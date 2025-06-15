//! In-memory event store for testing and development

use crate::event::{EventStore, SnapshotStore, TwinEvent, TwinSnapshot};
use crate::twin::TwinId;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// In-memory event store (non-persistent)
#[derive(Clone)]
pub struct MemoryEventStore {
    events: Arc<DashMap<u64, TwinEvent>>,
    twin_events: Arc<DashMap<TwinId, Vec<u64>>>,
    snapshots: Arc<DashMap<TwinId, TwinSnapshot>>,
    version_counter: Arc<AtomicU64>,
}

impl MemoryEventStore {
    /// Create a new in-memory event store
    pub fn new() -> Self {
        Self {
            events: Arc::new(DashMap::new()),
            twin_events: Arc::new(DashMap::new()),
            snapshots: Arc::new(DashMap::new()),
            version_counter: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl Default for MemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventStore for MemoryEventStore {
    async fn append(&self, event: TwinEvent) -> Result<u64> {
        let version = self.version_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let twin_id = event.twin_id();

        self.events.insert(version, event);

        self.twin_events.entry(twin_id).or_default().push(version);

        Ok(version)
    }

    async fn get_events(
        &self,
        twin_id: TwinId,
        after_version: u64,
    ) -> Result<Vec<(u64, TwinEvent)>> {
        let versions = self
            .twin_events
            .get(&twin_id)
            .map(|v| v.clone())
            .unwrap_or_default();

        let mut events = Vec::new();
        for version in versions {
            if version > after_version {
                if let Some(event) = self.events.get(&version) {
                    events.push((version, event.clone()));
                }
            }
        }

        events.sort_by_key(|(v, _)| *v);
        Ok(events)
    }

    async fn get_events_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<(u64, TwinEvent)>> {
        let mut events = Vec::new();

        for entry in self.events.iter() {
            let version = *entry.key();
            let event = entry.value();
            let timestamp = event.timestamp();

            if timestamp >= start && timestamp <= end {
                events.push((version, event.clone()));
            }
        }

        events.sort_by_key(|(v, _)| *v);
        Ok(events)
    }

    async fn get_latest_version(&self) -> Result<u64> {
        Ok(self.version_counter.load(Ordering::SeqCst))
    }
}

#[async_trait]
impl SnapshotStore for MemoryEventStore {
    async fn save_snapshot(&self, snapshot: TwinSnapshot) -> Result<()> {
        self.snapshots.insert(snapshot.twin_id, snapshot);
        Ok(())
    }

    async fn get_snapshot(&self, twin_id: TwinId) -> Result<Option<TwinSnapshot>> {
        Ok(self.snapshots.get(&twin_id).map(|s| s.clone()))
    }

    async fn cleanup_old_snapshots(&self, before: DateTime<Utc>) -> Result<u64> {
        let mut count = 0;
        let mut to_remove = Vec::new();

        for entry in self.snapshots.iter() {
            if entry.value().timestamp < before {
                to_remove.push(*entry.key());
                count += 1;
            }
        }

        for id in to_remove {
            self.snapshots.remove(&id);
        }

        Ok(count)
    }
}
