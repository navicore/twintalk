//! Tests for event store implementations

use chrono::{Duration, Utc};
use std::collections::BTreeMap;
use twintalk_core::event::{EventStore, SnapshotStore, TwinEvent, TwinSnapshot};
use twintalk_core::storage::memory_store::MemoryEventStore;
use twintalk_core::twin::TwinId;
use twintalk_core::Value;

#[tokio::test]
async fn test_memory_event_store() {
    let store = MemoryEventStore::new();
    let twin_id = TwinId::new();

    // Append events
    let created_event = TwinEvent::Created {
        twin_id,
        class_name: "Sensor".to_string(),
        timestamp: Utc::now(),
    };

    let version1 = store.append(created_event).await.unwrap();
    assert_eq!(version1, 1);

    let property_event = TwinEvent::PropertyChanged {
        twin_id,
        property: "temperature".to_string(),
        old_value: None,
        new_value: Value::from(25.0),
        timestamp: Utc::now(),
    };

    let version2 = store.append(property_event).await.unwrap();
    assert_eq!(version2, 2);

    // Get events for twin
    let events = store.get_events(twin_id, 0).await.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].0, 1); // version
    assert_eq!(events[1].0, 2);

    // Get events after version
    let events = store.get_events(twin_id, 1).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, 2);
}

#[tokio::test]
async fn test_event_ordering() {
    let store = MemoryEventStore::new();
    let twin_id = TwinId::new();

    // Append multiple events
    for i in 0..10 {
        let event = TwinEvent::PropertyChanged {
            twin_id,
            property: "value".to_string(),
            old_value: if i > 0 {
                Some(Value::Integer(i - 1))
            } else {
                None
            },
            new_value: Value::Integer(i),
            timestamp: Utc::now(),
        };
        store.append(event).await.unwrap();
    }

    // Events should be returned in order
    let events = store.get_events(twin_id, 0).await.unwrap();
    assert_eq!(events.len(), 10);

    for (i, (version, _)) in events.iter().enumerate() {
        assert_eq!(*version, (i + 1) as u64);
    }
}

#[tokio::test]
async fn test_event_time_range() {
    let store = MemoryEventStore::new();
    let start_time = Utc::now();

    // Create events at different times
    for i in 0..5 {
        let event = TwinEvent::Created {
            twin_id: TwinId::new(),
            class_name: format!("Sensor{i}"),
            timestamp: start_time + Duration::seconds(i),
        };
        store.append(event).await.unwrap();
    }

    // Query middle time range
    let range_start = start_time + Duration::seconds(1);
    let range_end = start_time + Duration::seconds(3);

    let events = store
        .get_events_in_range(range_start, range_end)
        .await
        .unwrap();
    assert_eq!(events.len(), 3); // Events at seconds 1, 2, and 3
}

#[tokio::test]
async fn test_snapshot_store() {
    let store = MemoryEventStore::new();
    let twin_id = TwinId::new();

    // Create snapshot
    let mut properties = BTreeMap::new();
    properties.insert("temperature".to_string(), Value::from(25.0));
    properties.insert("humidity".to_string(), Value::from(60.0));

    let snapshot = TwinSnapshot {
        twin_id,
        class_name: "Sensor".to_string(),
        properties,
        parent_id: None,
        event_version: 10,
        timestamp: Utc::now(),
    };

    // Save snapshot
    store.save_snapshot(snapshot.clone()).await.unwrap();

    // Retrieve snapshot
    let retrieved = store.get_snapshot(twin_id).await.unwrap();
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.twin_id, twin_id);
    assert_eq!(retrieved.class_name, "Sensor");
    assert_eq!(retrieved.event_version, 10);
}

#[tokio::test]
async fn test_snapshot_cleanup() {
    let store = MemoryEventStore::new();
    let now = Utc::now();

    // Create old and new snapshots
    for i in 0..5 {
        let snapshot = TwinSnapshot {
            twin_id: TwinId::new(),
            class_name: "Sensor".to_string(),
            properties: BTreeMap::new(),
            parent_id: None,
            event_version: i,
            timestamp: now - Duration::days(10_i64.saturating_sub(i64::try_from(i).unwrap_or(0))), // Older snapshots have older timestamps
        };
        store.save_snapshot(snapshot).await.unwrap();
    }

    // Clean up snapshots older than 5 days
    let cutoff = now - Duration::days(5);
    let deleted = store.cleanup_old_snapshots(cutoff).await.unwrap();

    // All snapshots are older than 5 days, so all should be deleted
    assert_eq!(deleted, 5);
}
