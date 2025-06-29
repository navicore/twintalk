//! `Sled`-based event store implementation
//!
//! Uses an embedded database for persistent event storage.

use crate::event::{EventStore, SnapshotStore, TwinEvent, TwinSnapshot};
use crate::twin::TwinId;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sled::{Db, Tree};
use std::sync::atomic::{AtomicU64, Ordering};

/// `Sled`-based persistent event store
pub struct SledEventStore {
    db: Db,
    events: Tree,
    snapshots: Tree,
    twin_events: Tree, // Index: twin_id -> event_ids
    version_counter: AtomicU64,
}

impl SledEventStore {
    /// Create a new `Sled` event store
    pub fn new(path: &str) -> Result<Self> {
        let db = sled::open(path).map_err(|e| anyhow!(e))?;
        let events = db.open_tree("events").map_err(|e| anyhow!(e))?;
        let snapshots = db.open_tree("snapshots").map_err(|e| anyhow!(e))?;
        let twin_events = db.open_tree("twin_events").map_err(|e| anyhow!(e))?;

        // Initialize version counter
        let latest_version = events
            .last()
            .map_err(|e| anyhow!(e))?
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
    fn index_event(&self, twin_id: TwinId, version: u64) -> Result<()> {
        let twin_key = twin_id.0.as_bytes();
        let _version_bytes = version.to_be_bytes();

        // Get existing versions for this twin
        let mut versions =
            if let Some(data) = self.twin_events.get(twin_key).map_err(|e| anyhow!(e))? {
                bincode::serde::decode_from_slice::<Vec<u64>, _>(&data, bincode::config::standard())
                    .map(|(decoded, _)| decoded)
                    .map_err(|e| anyhow!(e))?
            } else {
                Vec::new()
            };

        versions.push(version);

        let encoded = bincode::serde::encode_to_vec(&versions, bincode::config::standard())
            .map_err(|e| anyhow!(e))?;
        self.twin_events
            .insert(twin_key, encoded)
            .map_err(|e| anyhow!(e))?;

        Ok(())
    }
}

#[async_trait]
impl EventStore for SledEventStore {
    async fn append(&self, event: TwinEvent) -> Result<u64> {
        let version = self.version_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let version_bytes = version.to_be_bytes();

        let encoded = bincode::serde::encode_to_vec(&event, bincode::config::standard())
            .map_err(|e| anyhow!(e))?;

        self.events
            .insert(version_bytes, encoded)
            .map_err(|e| anyhow!(e))?;

        // Index by twin
        self.index_event(event.twin_id(), version)?;

        // Flush to ensure durability
        self.db.flush_async().await.map_err(|e| anyhow!(e))?;

        Ok(version)
    }

    async fn get_events(
        &self,
        twin_id: TwinId,
        after_version: u64,
    ) -> Result<Vec<(u64, TwinEvent)>> {
        let twin_key = twin_id.0.as_bytes();

        // Get all versions for this twin
        let versions = if let Some(data) = self.twin_events.get(twin_key).map_err(|e| anyhow!(e))? {
            bincode::serde::decode_from_slice::<Vec<u64>, _>(&data, bincode::config::standard())
                .map(|(decoded, _)| decoded)
                .map_err(|e| anyhow!(e))?
        } else {
            return Ok(vec![]);
        };

        let mut events = Vec::new();

        for version in versions {
            if version > after_version {
                let version_bytes = version.to_be_bytes();
                if let Some(data) = self.events.get(version_bytes).map_err(|e| anyhow!(e))? {
                    let event: TwinEvent =
                        bincode::serde::decode_from_slice(&data, bincode::config::standard())
                            .map(|(decoded, _)| decoded)
                            .map_err(|e| anyhow!(e))?;
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
    ) -> Result<Vec<(u64, TwinEvent)>> {
        let mut events = Vec::new();

        for item in &self.events {
            let (key, value) = item.map_err(|e| anyhow!(e))?;
            let version = u64::from_be_bytes(
                key.as_ref()
                    .try_into()
                    .map_err(|_| anyhow!("Invalid key"))?,
            );
            let event: TwinEvent =
                bincode::serde::decode_from_slice(&value, bincode::config::standard())
                    .map(|(decoded, _)| decoded)
                    .map_err(|e| anyhow!(e))?;

            let timestamp = event.timestamp();
            if timestamp >= start && timestamp <= end {
                events.push((version, event));
            }
        }

        Ok(events)
    }

    async fn get_latest_version(&self) -> Result<u64> {
        Ok(self.version_counter.load(Ordering::SeqCst))
    }
}

#[async_trait]
impl SnapshotStore for SledEventStore {
    async fn save_snapshot(&self, snapshot: TwinSnapshot) -> Result<()> {
        let key = snapshot.twin_id.0.as_bytes();
        let encoded = bincode::serde::encode_to_vec(&snapshot, bincode::config::standard())
            .map_err(|e| anyhow!(e))?;

        self.snapshots
            .insert(key, encoded)
            .map_err(|e| anyhow!(e))?;
        self.db.flush_async().await.map_err(|e| anyhow!(e))?;

        Ok(())
    }

    async fn get_snapshot(&self, twin_id: TwinId) -> Result<Option<TwinSnapshot>> {
        let key = twin_id.0.as_bytes();

        if let Some(data) = self.snapshots.get(key).map_err(|e| anyhow!(e))? {
            let snapshot = bincode::serde::decode_from_slice(&data, bincode::config::standard())
                .map(|(decoded, _)| decoded)
                .map_err(|e| anyhow!(e))?;
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }

    async fn cleanup_old_snapshots(&self, before: DateTime<Utc>) -> Result<u64> {
        let mut count = 0;
        let mut to_remove = Vec::new();

        for item in &self.snapshots {
            let (key, value) = item.map_err(|e| anyhow!(e))?;
            let snapshot: TwinSnapshot =
                bincode::serde::decode_from_slice(&value, bincode::config::standard())
                    .map(|(decoded, _)| decoded)
                    .map_err(|e| anyhow!(e))?;

            if snapshot.timestamp < before {
                to_remove.push(key);
                count += 1;
            }
        }

        for key in to_remove {
            self.snapshots.remove(key).map_err(|e| anyhow!(e))?;
        }

        self.db.flush_async().await.map_err(|e| anyhow!(e))?;
        Ok(count)
    }
}
