//! Sled-based event store implementation
//!
//! Uses an embedded database for persistent event storage.

use crate::event::{EventStore, SnapshotStore, TwinEvent, TwinSnapshot};
use crate::twin::TwinId;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sled::{Db, Tree};
use std::sync::atomic::{AtomicU64, Ordering};

/// Sled-based persistent event store
pub struct SledEventStore {
    db: Db,
    events: Tree,
    snapshots: Tree,
    twin_events: Tree, // Index: twin_id -> event_ids
    version_counter: AtomicU64,
}

impl SledEventStore {
    /// Create a new Sled event store
    pub fn new(path: &str) -> Result<Self, String> {
        let db = sled::open(path).map_err(|e| e.to_string())?;
        let events = db.open_tree("events").map_err(|e| e.to_string())?;
        let snapshots = db.open_tree("snapshots").map_err(|e| e.to_string())?;
        let twin_events = db.open_tree("twin_events").map_err(|e| e.to_string())?;

        // Initialize version counter
        let latest_version = events
            .last()
            .map_err(|e| e.to_string())?
            .and_then(|(k, _)| {
                let bytes: [u8; 8] = k.as_ref().try_into().ok()?;
                Some(u64::from_be_bytes(bytes))
            })
            .unwrap_or(0);

        Ok(Self {
            db,
            events,
            snapshots,
            twin_events,
            version_counter: AtomicU64::new(latest_version),
        })
    }

    /// Helper to add event to twin index
    async fn index_event(&self, twin_id: TwinId, version: u64) -> Result<(), String> {
        let twin_key = twin_id.0.as_bytes();
        let _version_bytes = version.to_be_bytes();

        // Get existing versions for this twin
        let mut versions =
            if let Some(data) = self.twin_events.get(twin_key).map_err(|e| e.to_string())? {
                bincode::deserialize::<Vec<u64>>(&data).map_err(|e| e.to_string())?
            } else {
                Vec::new()
            };

        versions.push(version);

        let encoded = bincode::serialize(&versions).map_err(|e| e.to_string())?;
        self.twin_events
            .insert(twin_key, encoded)
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

#[async_trait]
impl EventStore for SledEventStore {
    async fn append(&self, event: TwinEvent) -> Result<u64, String> {
        let version = self.version_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let version_bytes = version.to_be_bytes();

        let encoded = bincode::serialize(&event).map_err(|e| e.to_string())?;

        self.events
            .insert(version_bytes, encoded)
            .map_err(|e| e.to_string())?;

        // Index by twin
        self.index_event(event.twin_id(), version).await?;

        // Flush to ensure durability
        self.db.flush_async().await.map_err(|e| e.to_string())?;

        Ok(version)
    }

    async fn get_events(
        &self,
        twin_id: TwinId,
        after_version: u64,
    ) -> Result<Vec<(u64, TwinEvent)>, String> {
        let twin_key = twin_id.0.as_bytes();

        // Get all versions for this twin
        let versions =
            if let Some(data) = self.twin_events.get(twin_key).map_err(|e| e.to_string())? {
                bincode::deserialize::<Vec<u64>>(&data).map_err(|e| e.to_string())?
            } else {
                return Ok(vec![]);
            };

        let mut events = Vec::new();

        for version in versions {
            if version > after_version {
                let version_bytes = version.to_be_bytes();
                if let Some(data) = self.events.get(version_bytes).map_err(|e| e.to_string())? {
                    let event: TwinEvent =
                        bincode::deserialize(&data).map_err(|e| e.to_string())?;
                    events.push((version, event));
                }
            }
        }

        Ok(events)
    }

    async fn get_events_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<(u64, TwinEvent)>, String> {
        let mut events = Vec::new();

        for item in self.events.iter() {
            let (key, value) = item.map_err(|e| e.to_string())?;
            let version = u64::from_be_bytes(key.as_ref().try_into().map_err(|_| "Invalid key")?);
            let event: TwinEvent = bincode::deserialize(&value).map_err(|e| e.to_string())?;

            let timestamp = event.timestamp();
            if timestamp >= start && timestamp <= end {
                events.push((version, event));
            }
        }

        Ok(events)
    }

    async fn get_latest_version(&self) -> Result<u64, String> {
        Ok(self.version_counter.load(Ordering::SeqCst))
    }
}

#[async_trait]
impl SnapshotStore for SledEventStore {
    async fn save_snapshot(&self, snapshot: TwinSnapshot) -> Result<(), String> {
        let key = snapshot.twin_id.0.as_bytes();
        let encoded = bincode::serialize(&snapshot).map_err(|e| e.to_string())?;

        self.snapshots
            .insert(key, encoded)
            .map_err(|e| e.to_string())?;
        self.db.flush_async().await.map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn get_snapshot(&self, twin_id: TwinId) -> Result<Option<TwinSnapshot>, String> {
        let key = twin_id.0.as_bytes();

        if let Some(data) = self.snapshots.get(key).map_err(|e| e.to_string())? {
            let snapshot = bincode::deserialize(&data).map_err(|e| e.to_string())?;
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }

    async fn cleanup_old_snapshots(&self, before: DateTime<Utc>) -> Result<u64, String> {
        let mut count = 0;
        let mut to_remove = Vec::new();

        for item in self.snapshots.iter() {
            let (key, value) = item.map_err(|e| e.to_string())?;
            let snapshot: TwinSnapshot = bincode::deserialize(&value).map_err(|e| e.to_string())?;

            if snapshot.timestamp < before {
                to_remove.push(key);
                count += 1;
            }
        }

        for key in to_remove {
            self.snapshots.remove(key).map_err(|e| e.to_string())?;
        }

        self.db.flush_async().await.map_err(|e| e.to_string())?;
        Ok(count)
    }
}
