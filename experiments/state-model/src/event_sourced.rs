//! Event-sourced state model for digital twins
//! 
//! Explores lazy instantiation from event logs with snapshot optimization

use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct TwinId(pub Uuid);

impl TwinId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

// Events that can happen to a twin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TwinEvent {
    Created {
        twin_id: TwinId,
        class_name: String,
        timestamp: DateTime<Utc>,
    },
    PropertySet {
        twin_id: TwinId,
        property: String,
        value: serde_json::Value,
        timestamp: DateTime<Utc>,
    },
    TelemetryReceived {
        twin_id: TwinId,
        data: serde_json::Value,
        timestamp: DateTime<Utc>,
    },
    MessageSent {
        twin_id: TwinId,
        selector: String,
        args: Vec<serde_json::Value>,
        timestamp: DateTime<Utc>,
    },
    Cloned {
        twin_id: TwinId,
        source_id: TwinId,
        timestamp: DateTime<Utc>,
    },
}

// Snapshot for faster reconstruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinSnapshot {
    pub twin_id: TwinId,
    pub class_name: String,
    pub properties: serde_json::Map<String, serde_json::Value>,
    pub event_version: u64,
    pub timestamp: DateTime<Utc>,
}

// Lightweight twin state (what's in memory)
#[derive(Debug, Clone)]
pub struct TwinState {
    pub id: TwinId,
    pub class_name: String,
    pub properties: serde_json::Map<String, serde_json::Value>,
    pub last_event_version: u64,
    pub last_accessed: Instant,
}

impl TwinState {
    fn new(id: TwinId, class_name: String) -> Self {
        Self {
            id,
            class_name,
            properties: serde_json::Map::new(),
            last_event_version: 0,
            last_accessed: Instant::now(),
        }
    }

    fn apply_event(&mut self, event: &TwinEvent) -> Result<(), String> {
        match event {
            TwinEvent::PropertySet { property, value, .. } => {
                self.properties.insert(property.clone(), value.clone());
                self.last_event_version += 1;
            }
            TwinEvent::TelemetryReceived { data, .. } => {
                if let serde_json::Value::Object(map) = data {
                    for (k, v) in map {
                        self.properties.insert(k.clone(), v.clone());
                    }
                }
                self.last_event_version += 1;
            }
            _ => {}
        }
        self.last_accessed = Instant::now();
        Ok(())
    }
}

// Event store abstraction
trait EventStore: Send + Sync {
    fn append(&self, event: TwinEvent) -> Result<u64, String>;
    fn get_events(&self, twin_id: &TwinId, after_version: u64) -> Result<Vec<TwinEvent>, String>;
    fn save_snapshot(&self, snapshot: TwinSnapshot) -> Result<(), String>;
    fn get_snapshot(&self, twin_id: &TwinId) -> Result<Option<TwinSnapshot>, String>;
}

// Simple in-memory implementation for testing
struct MemoryEventStore {
    events: DashMap<TwinId, Vec<TwinEvent>>,
    snapshots: DashMap<TwinId, TwinSnapshot>,
}

impl MemoryEventStore {
    fn new() -> Self {
        Self {
            events: DashMap::new(),
            snapshots: DashMap::new(),
        }
    }
}

impl EventStore for MemoryEventStore {
    fn append(&self, event: TwinEvent) -> Result<u64, String> {
        let twin_id = match &event {
            TwinEvent::Created { twin_id, .. } => twin_id,
            TwinEvent::PropertySet { twin_id, .. } => twin_id,
            TwinEvent::TelemetryReceived { twin_id, .. } => twin_id,
            TwinEvent::MessageSent { twin_id, .. } => twin_id,
            TwinEvent::Cloned { twin_id, .. } => twin_id,
        };
        
        let mut events = self.events.entry(twin_id.clone()).or_insert_with(Vec::new);
        events.push(event);
        Ok(events.len() as u64)
    }

    fn get_events(&self, twin_id: &TwinId, after_version: u64) -> Result<Vec<TwinEvent>, String> {
        Ok(self.events
            .get(twin_id)
            .map(|events| events.iter()
                .skip(after_version as usize)
                .cloned()
                .collect())
            .unwrap_or_default())
    }

    fn save_snapshot(&self, snapshot: TwinSnapshot) -> Result<(), String> {
        self.snapshots.insert(snapshot.twin_id.clone(), snapshot);
        Ok(())
    }

    fn get_snapshot(&self, twin_id: &TwinId) -> Result<Option<TwinSnapshot>, String> {
        Ok(self.snapshots.get(twin_id).map(|s| s.clone()))
    }
}

// The runtime that manages lazy loading
pub struct TwinRuntime {
    event_store: Arc<dyn EventStore>,
    active_twins: DashMap<TwinId, Arc<TwinState>>,
    eviction_interval: Duration,
}

impl TwinRuntime {
    fn new(event_store: Arc<dyn EventStore>) -> Self {
        Self {
            event_store,
            active_twins: DashMap::new(),
            eviction_interval: Duration::from_secs(60), // Evict after 60s of inactivity
        }
    }

    // Lazily load or get active twin
    async fn get_twin(&self, twin_id: &TwinId) -> Result<Arc<TwinState>, String> {
        // Check if already active
        if let Some(twin) = self.active_twins.get(twin_id) {
            return Ok(twin.clone());
        }

        // Load from event store
        self.load_twin(twin_id).await
    }

    async fn load_twin(&self, twin_id: &TwinId) -> Result<Arc<TwinState>, String> {
        // Try to load from snapshot first
        let (mut state, start_version) = if let Some(snapshot) = self.event_store.get_snapshot(twin_id)? {
            let state = TwinState {
                id: snapshot.twin_id,
                class_name: snapshot.class_name,
                properties: snapshot.properties,
                last_event_version: snapshot.event_version,
                last_accessed: Instant::now(),
            };
            (state, snapshot.event_version)
        } else {
            // No snapshot, need to replay from beginning
            (TwinState::new(twin_id.clone(), String::new()), 0)
        };

        // Replay events after snapshot
        let events = self.event_store.get_events(twin_id, start_version)?;
        for event in events {
            state.apply_event(&event)?;
        }

        let state = Arc::new(state);
        self.active_twins.insert(twin_id.clone(), state.clone());
        Ok(state)
    }

    // Process telemetry - this is where lazy loading happens
    async fn update_telemetry(&self, twin_id: &TwinId, data: serde_json::Value) -> Result<(), String> {
        // Append event first (for durability)
        let event = TwinEvent::TelemetryReceived {
            twin_id: twin_id.clone(),
            data: data.clone(),
            timestamp: Utc::now(),
        };
        self.event_store.append(event.clone())?;

        // Then update in-memory state if active
        if let Some(mut twin) = self.active_twins.get_mut(twin_id) {
            // Clone and update
            let mut new_state = (**twin).clone();
            new_state.apply_event(&event)?;
            *twin = Arc::new(new_state);
        }
        // If not active, we don't load it - true lazy loading!
        
        Ok(())
    }

    // Create snapshot for active twins (periodic background task)
    async fn snapshot_twin(&self, twin_id: &TwinId) -> Result<(), String> {
        if let Some(twin) = self.active_twins.get(twin_id) {
            let snapshot = TwinSnapshot {
                twin_id: twin.id.clone(),
                class_name: twin.class_name.clone(),
                properties: twin.properties.clone(),
                event_version: twin.last_event_version,
                timestamp: Utc::now(),
            };
            self.event_store.save_snapshot(snapshot)?;
        }
        Ok(())
    }

    // Evict inactive twins from memory
    async fn evict_inactive(&self) {
        let now = Instant::now();
        let mut to_evict = Vec::new();

        for entry in self.active_twins.iter() {
            if now.duration_since(entry.value().last_accessed) > self.eviction_interval {
                to_evict.push(entry.key().clone());
            }
        }

        for twin_id in to_evict {
            // Snapshot before evicting
            self.snapshot_twin(&twin_id).await.ok();
            self.active_twins.remove(&twin_id);
        }
    }
}

#[tokio::main]
async fn main() {
    println!("=== Event-Sourced Twin State Model ===\n");

    let event_store = Arc::new(MemoryEventStore::new());
    let runtime = Arc::new(TwinRuntime::new(event_store.clone()));

    // Create some twins
    let mut twin_ids = Vec::new();
    for i in 0..1000 {
        let twin_id = TwinId::new();
        twin_ids.push(twin_id.clone());
        
        // Create event
        event_store.append(TwinEvent::Created {
            twin_id: twin_id.clone(),
            class_name: "TemperatureSensor".to_string(),
            timestamp: Utc::now(),
        }).unwrap();
        
        // Initial properties
        event_store.append(TwinEvent::PropertySet {
            twin_id: twin_id.clone(),
            property: "temperature".to_string(),
            value: serde_json::json!(20.0),
            timestamp: Utc::now(),
        }).unwrap();
    }

    println!("Created 1000 twins (events only, not in memory)");
    println!("Active twins in memory: {}", runtime.active_twins.len());

    // Simulate telemetry for only 10% of twins
    println!("\nSending telemetry to 100 twins...");
    let start = Instant::now();
    
    for i in 0..100 {
        let data = serde_json::json!({
            "temperature": 20.0 + (i as f64 * 0.1),
            "humidity": 45.0 + (i as f64 * 0.5)
        });
        runtime.update_telemetry(&twin_ids[i], data).await.unwrap();
    }
    
    println!("Telemetry processing time: {:?}", start.elapsed());
    println!("Active twins in memory: {}", runtime.active_twins.len());

    // Now access some twins that weren't updated (lazy load)
    println!("\nLazy loading twin 500...");
    let start = Instant::now();
    let twin = runtime.get_twin(&twin_ids[500]).await.unwrap();
    println!("Load time: {:?}", start.elapsed());
    println!("Twin state: {:?}", twin.properties);
    println!("Active twins in memory: {}", runtime.active_twins.len());

    // Simulate eviction
    println!("\nSimulating eviction after inactivity...");
    runtime.evict_inactive().await;
    println!("Active twins after eviction: {}", runtime.active_twins.len());

    // Performance comparison with actor model
    println!("\n=== Performance Comparison ===");
    println!("Event-sourced approach:");
    println!("- No actor overhead (no mailboxes, supervisors)");
    println!("- Lazy loading only when needed");
    println!("- Events persisted immediately");
    println!("- Memory footprint: ~{} bytes per active twin", 
        std::mem::size_of::<TwinState>());
    
    println!("\nActor model (Akka-style) would have:");
    println!("- ~8KB per actor (JVM overhead)");
    println!("- Mailbox memory overhead");
    println!("- Thread pool management");
    println!("- Supervision tree overhead");
}