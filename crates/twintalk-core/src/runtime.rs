//! Twin runtime with lazy loading and event sourcing
//!
//! Manages the lifecycle of twins with efficient memory usage.

use crate::event::{EventStore, SnapshotStore, TwinEvent, TwinSnapshot};
use crate::storage::memory_store::MemoryEventStore;
use crate::twin::{Twin, TwinId, TwinState};
use crate::value::Value;
use anyhow::{anyhow, Result};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Configuration for the runtime
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// How long to keep twins in memory after last access
    pub eviction_timeout: Duration,

    /// How often to run the eviction task
    pub eviction_interval: Duration,

    /// Whether to create snapshots on eviction
    pub snapshot_on_eviction: bool,

    /// Maximum number of active twins in memory
    pub max_active_twins: Option<usize>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            eviction_timeout: Duration::from_secs(300), // 5 minutes
            eviction_interval: Duration::from_secs(60), // Check every minute
            snapshot_on_eviction: true,
            max_active_twins: None,
        }
    }
}

/// Active twin wrapper with last access tracking
pub struct ActiveTwin {
    pub twin: RwLock<Twin>,
    last_accessed: RwLock<Instant>,
}

impl ActiveTwin {
    fn new(twin: Twin) -> Self {
        Self {
            twin: RwLock::new(twin),
            last_accessed: RwLock::new(Instant::now()),
        }
    }

    async fn touch(&self) {
        *self.last_accessed.write().await = Instant::now();
    }
}

/// The main runtime for managing twins
pub struct Runtime {
    config: RuntimeConfig,
    event_store: Arc<dyn EventStore>,
    snapshot_store: Arc<dyn SnapshotStore>,
    active_twins: Arc<DashMap<TwinId, Arc<ActiveTwin>>>,
}

impl Runtime {
    /// Create a new runtime with the given configuration
    pub fn new(config: RuntimeConfig) -> Self {
        let store = Arc::new(MemoryEventStore::new());
        Self {
            config,
            event_store: store.clone(),
            snapshot_store: store,
            active_twins: Arc::new(DashMap::new()),
        }
    }

    /// Create a runtime with custom stores
    pub fn with_stores(
        config: RuntimeConfig,
        event_store: Arc<dyn EventStore>,
        snapshot_store: Arc<dyn SnapshotStore>,
    ) -> Self {
        Self {
            config,
            event_store,
            snapshot_store,
            active_twins: Arc::new(DashMap::new()),
        }
    }

    /// Create a new twin
    pub async fn create_twin(&self, class_name: impl Into<String>) -> Result<TwinId> {
        let twin = Twin::new(class_name.into());
        let twin_id = twin.id();

        // Record creation event
        let event = TwinEvent::Created {
            twin_id,
            class_name: twin.class_name().to_string(),
            timestamp: Utc::now(),
        };
        self.event_store.append(event).await?;

        // Add to active twins
        self.active_twins
            .insert(twin_id, Arc::new(ActiveTwin::new(twin)));

        Ok(twin_id)
    }

    /// Get or load a twin
    pub async fn get_twin(&self, twin_id: TwinId) -> Result<Arc<ActiveTwin>> {
        // Check if already active
        if let Some(twin) = self.active_twins.get(&twin_id) {
            twin.touch().await;
            return Ok(twin.clone());
        }

        // Load from persistence
        self.load_twin(twin_id).await
    }

    /// Create a hypothetical twin (not persisted)
    pub async fn create_hypothetical_twin(&self, class_name: &str) -> Result<TwinId> {
        let mut twin = Twin::new(class_name);
        twin.state.is_hypothetical = true;
        twin.state.simulation_time = Some(Utc::now());
        
        let id = twin.id();
        let active = Arc::new(ActiveTwin::new(twin));
        self.active_twins.insert(id, active);
        
        Ok(id)
    }

    /// Load a twin from events/snapshots
    async fn load_twin(&self, twin_id: TwinId) -> Result<Arc<ActiveTwin>> {
        // Try to load from snapshot first
        let (state, start_version) =
            if let Some(snapshot) = self.snapshot_store.get_snapshot(twin_id).await? {
                let state = TwinState {
                    id: snapshot.twin_id,
                    class_name: snapshot.class_name,
                    properties: snapshot.properties,
                    parent_id: snapshot.parent_id,
                    created_at: snapshot.timestamp,
                    updated_at: snapshot.timestamp,
                    is_hypothetical: false,
                    simulation_time: None,
                };
                (Some(state), snapshot.event_version)
            } else {
                (None, 0)
            };

        // Replay events after snapshot
        let events = self.event_store.get_events(twin_id, start_version).await?;

        if events.is_empty() && state.is_none() {
            return Err(anyhow!("Twin {twin_id} not found"));
        }

        // Create twin from first event if no snapshot
        let had_snapshot = state.is_some();
        let mut twin = if let Some(s) = state {
            Twin::from_state(s)
        } else if let Some((_, first_event)) = events.first() {
            match first_event {
                TwinEvent::Created { class_name, .. } => Twin::new(class_name.clone()),
                _ => return Err(anyhow!("First event must be Created")),
            }
        } else {
            return Err(anyhow!("No state or events found"));
        };

        // Replay remaining events
        for (_, event) in events.iter().skip(usize::from(!had_snapshot)) {
            Self::apply_event(&mut twin, event)?;
        }

        let active = Arc::new(ActiveTwin::new(twin));
        self.active_twins.insert(twin_id, active.clone());

        Ok(active)
    }

    /// Apply an event to a twin
    fn apply_event(twin: &mut Twin, event: &TwinEvent) -> Result<()> {
        match event {
            TwinEvent::PropertyChanged {
                property,
                new_value,
                ..
            } => {
                twin.send(&crate::message::Message::SetProperty(
                    property.clone(),
                    new_value.clone(),
                ))?;
            }
            TwinEvent::TelemetryReceived { data, .. } => {
                let updates: Vec<_> = data
                    .iter()
                    .map(|(k, v)| (k.clone(), Value::Float((*v).into())))
                    .collect();
                twin.send(&crate::message::Message::UpdateProperties(updates))?;
            }
            _ => {} // Other events don't modify state
        }
        Ok(())
    }

    /// Update twin with telemetry
    pub async fn update_telemetry(&self, twin_id: TwinId, data: Vec<(String, f64)>) -> Result<()> {
        // Check if twin is hypothetical - if so, skip persistence
        let is_hypothetical = if let Some(active) = self.active_twins.get(&twin_id) {
            let twin = active.twin.read().await;
            twin.is_hypothetical()
        } else {
            false
        };

        // Only persist events for non-hypothetical twins
        if !is_hypothetical {
            let event = TwinEvent::TelemetryReceived {
                twin_id,
                data: data.clone(),
                timestamp: Utc::now(),
            };
            self.event_store.append(event).await?;
        }

        // Update in-memory twin if active
        if let Some(active) = self.active_twins.get(&twin_id) {
            active.touch().await;
            let updates: Vec<_> = data
                .into_iter()
                .map(|(k, v)| (k, Value::Float(v.into())))
                .collect();
            let mut twin = active.twin.write().await;
            twin.send(&crate::message::Message::UpdateProperties(updates))?;
        }
        // If not active, we don't load it - true lazy loading!

        Ok(())
    }

    /// Create a snapshot for a twin
    pub async fn snapshot_twin(&self, twin_id: TwinId) -> Result<()> {
        let active = self.get_twin(twin_id).await?;

        let (class_name, properties, parent_id) = {
            let twin = active.twin.read().await;
            let state = twin.state();
            let class_name = state.class_name.clone();
            let properties = state.properties.clone();
            let parent_id = state.parent_id;
            drop(twin); // Explicitly drop the lock before the tuple is created
            (class_name, properties, parent_id)
        };

        let version = self.event_store.get_latest_version().await?;

        let snapshot = TwinSnapshot {
            twin_id,
            class_name,
            properties,
            parent_id,
            event_version: version,
            timestamp: Utc::now(),
        };

        self.snapshot_store.save_snapshot(snapshot).await?;
        Ok(())
    }

    /// Evict inactive twins from memory
    pub async fn evict_inactive(&self) -> Result<usize> {
        let now = Instant::now();
        let mut to_evict = Vec::new();

        for entry in self.active_twins.iter() {
            let last_accessed = *entry.value().last_accessed.read().await;
            if now.duration_since(last_accessed) > self.config.eviction_timeout {
                to_evict.push(*entry.key());
            }
        }

        let count = to_evict.len();

        for twin_id in to_evict {
            if self.config.snapshot_on_eviction {
                self.snapshot_twin(twin_id).await.ok();
            }
            self.active_twins.remove(&twin_id);
        }

        Ok(count)
    }

    /// Start the background eviction task
    pub fn start_eviction_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(self.config.eviction_interval);
            loop {
                interval.tick().await;
                if let Ok(count) = self.evict_inactive().await {
                    if count > 0 {
                        tracing::debug!("Evicted {} inactive twins", count);
                    }
                }
            }
        });
    }

    /// Get runtime statistics
    pub async fn stats(&self) -> RuntimeStats {
        RuntimeStats {
            active_twins: self.active_twins.len(),
            total_events: self.event_store.get_latest_version().await.unwrap_or(0),
        }
    }
}

/// Runtime statistics
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    pub active_twins: usize,
    pub total_events: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_twin_lifecycle() {
        let runtime = Runtime::new(RuntimeConfig::default());

        // Create twin
        let twin_id = runtime.create_twin("Sensor").await.unwrap();

        // Update telemetry
        runtime
            .update_telemetry(twin_id, vec![("temperature".to_string(), 25.0)])
            .await
            .unwrap();

        // Get twin and verify
        let active = runtime.get_twin(twin_id).await.unwrap();
        let temp = {
            let mut twin = active.twin.write().await;
            twin.send(&crate::msg!(temperature)).unwrap()
        };
        assert_eq!(temp, Value::from(25.0));
    }
}
