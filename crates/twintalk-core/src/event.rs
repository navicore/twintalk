//! Event sourcing for twin persistence
//!
//! All twin state changes are recorded as events for replay and audit.

use crate::twin::TwinId;
use crate::value::Value;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Events that can happen to a twin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TwinEvent {
    /// Twin was created
    Created {
        twin_id: TwinId,
        class_name: String,
        timestamp: DateTime<Utc>,
    },

    /// Property was changed
    PropertyChanged {
        twin_id: TwinId,
        property: String,
        old_value: Option<Value>,
        new_value: Value,
        timestamp: DateTime<Utc>,
    },

    /// Telemetry was received
    TelemetryReceived {
        twin_id: TwinId,
        data: Vec<(String, f64)>,
        timestamp: DateTime<Utc>,
    },

    /// Message was sent
    MessageSent {
        twin_id: TwinId,
        selector: String,
        args: Vec<Value>,
        result: Result<Value, String>,
        timestamp: DateTime<Utc>,
    },

    /// Twin was cloned
    Cloned {
        twin_id: TwinId,
        source_id: TwinId,
        timestamp: DateTime<Utc>,
    },

    /// Twin was destroyed
    Destroyed {
        twin_id: TwinId,
        timestamp: DateTime<Utc>,
    },
}

impl TwinEvent {
    /// Get the twin ID this event applies to
    pub fn twin_id(&self) -> TwinId {
        match self {
            Self::Created { twin_id, .. }
            | Self::PropertyChanged { twin_id, .. }
            | Self::TelemetryReceived { twin_id, .. }
            | Self::MessageSent { twin_id, .. }
            | Self::Cloned { twin_id, .. }
            | Self::Destroyed { twin_id, .. } => *twin_id,
        }
    }

    /// Get the timestamp of this event
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::Created { timestamp, .. }
            | Self::PropertyChanged { timestamp, .. }
            | Self::TelemetryReceived { timestamp, .. }
            | Self::MessageSent { timestamp, .. }
            | Self::Cloned { timestamp, .. }
            | Self::Destroyed { timestamp, .. } => *timestamp,
        }
    }
}

impl fmt::Display for TwinEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created {
                twin_id,
                class_name,
                timestamp,
            } => {
                write!(f, "[{timestamp}] Created {twin_id} ({class_name})")
            }
            Self::PropertyChanged {
                twin_id,
                property,
                new_value,
                timestamp,
                ..
            } => {
                write!(
                    f,
                    "[{timestamp}] {twin_id} property '{property}' = {new_value}"
                )
            }
            Self::TelemetryReceived {
                twin_id,
                data,
                timestamp,
            } => {
                write!(
                    f,
                    "[{timestamp}] {twin_id} received {} telemetry values",
                    data.len()
                )
            }
            Self::MessageSent {
                twin_id,
                selector,
                timestamp,
                ..
            } => {
                write!(f, "[{timestamp}] {twin_id} received message '{selector}'")
            }
            Self::Cloned {
                twin_id,
                source_id,
                timestamp,
            } => {
                write!(f, "[{timestamp}] {twin_id} cloned from {source_id})")
            }
            Self::Destroyed { twin_id, timestamp } => {
                write!(f, "[{timestamp}] {twin_id} destroyed")
            }
        }
    }
}

/// Event store trait for different storage backends
#[async_trait::async_trait]
pub trait EventStore: Send + Sync {
    /// Append an event to the store
    async fn append(&self, event: TwinEvent) -> Result<u64>;

    /// Get all events for a twin after a certain version
    async fn get_events(
        &self,
        twin_id: TwinId,
        after_version: u64,
    ) -> Result<Vec<(u64, TwinEvent)>>;

    /// Get all events in a time range
    async fn get_events_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<(u64, TwinEvent)>>;

    /// Get the latest version number
    async fn get_latest_version(&self) -> Result<u64>;
}

/// Snapshot for faster twin reconstruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinSnapshot {
    pub twin_id: TwinId,
    pub class_name: String,
    pub properties: std::collections::BTreeMap<String, Value>,
    pub parent_id: Option<TwinId>,
    pub event_version: u64,
    pub timestamp: DateTime<Utc>,
}

/// Snapshot store trait
#[async_trait::async_trait]
pub trait SnapshotStore: Send + Sync {
    /// Save a snapshot
    async fn save_snapshot(&self, snapshot: TwinSnapshot) -> Result<()>;

    /// Get the latest snapshot for a twin
    async fn get_snapshot(&self, twin_id: TwinId) -> Result<Option<TwinSnapshot>>;

    /// Delete old snapshots
    async fn cleanup_old_snapshots(&self, before: DateTime<Utc>) -> Result<u64>;
}
