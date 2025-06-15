//! Integration tests for the complete runtime

use std::sync::Arc;
use std::time::Duration;
use twintalk_core::{msg, Message, Runtime, RuntimeConfig, TwinId, Value};

#[tokio::test]
async fn test_twin_lifecycle() {
    let runtime = Runtime::new(RuntimeConfig::default());

    // Create twin
    let twin_id = runtime.create_twin("TemperatureSensor").await.unwrap();

    // Send telemetry
    runtime
        .update_telemetry(
            twin_id,
            vec![
                ("temperature".to_string(), 25.0),
                ("threshold".to_string(), 30.0),
            ],
        )
        .await
        .unwrap();

    // Get twin and verify state
    let active = runtime.get_twin(twin_id).await.unwrap();
    {
        let mut twin = active.twin.write().await;
        assert_eq!(twin.send(&msg!(temperature)).unwrap(), Value::from(25.0));
        assert_eq!(twin.send(&msg!(threshold)).unwrap(), Value::from(30.0));
        drop(twin); // Explicitly drop the lock
    }
}

#[tokio::test]
async fn test_lazy_loading() {
    let runtime = Arc::new(Runtime::new(RuntimeConfig {
        eviction_timeout: Duration::from_millis(100),
        eviction_interval: Duration::from_secs(1),
        snapshot_on_eviction: true,
        max_active_twins: None,
    }));

    // Create twin
    let twin_id = runtime.create_twin("Sensor").await.unwrap();

    // Set initial state
    runtime
        .update_telemetry(twin_id, vec![("value".to_string(), 42.0)])
        .await
        .unwrap();

    // Stats should show 1 active twin
    let stats = runtime.stats().await;
    assert_eq!(stats.active_twins, 1);

    // Wait for eviction timeout to pass
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Evict twins manually
    runtime.evict_inactive().await.unwrap();

    // After eviction, no active twins
    let stats = runtime.stats().await;
    assert_eq!(stats.active_twins, 0);

    // Twin should be lazily loaded when accessed
    let active = runtime.get_twin(twin_id).await.unwrap();
    {
        let mut twin = active.twin.write().await;
        assert_eq!(twin.send(&msg!(value)).unwrap(), Value::from(42.0));
        drop(twin); // Explicitly drop the lock
    }

    // Stats should show 1 active twin again
    let stats = runtime.stats().await;
    assert_eq!(stats.active_twins, 1);
}

#[tokio::test]
async fn test_event_sourcing() {
    let runtime = Runtime::new(RuntimeConfig::default());

    // Create twin
    let twin_id = runtime.create_twin("EventedSensor").await.unwrap();

    // Send multiple telemetry updates
    for i in 0..5 {
        runtime
            .update_telemetry(twin_id, vec![("counter".to_string(), f64::from(i))])
            .await
            .unwrap();
    }

    // Get final state
    let active = runtime.get_twin(twin_id).await.unwrap();
    {
        let mut twin = active.twin.write().await;
        assert_eq!(twin.send(&msg!(counter)).unwrap(), Value::from(4.0));
        drop(twin); // Explicitly drop the lock
    }

    // Stats should show events
    let stats = runtime.stats().await;
    assert_eq!(stats.total_events, 6); // 1 create + 5 telemetry
}

#[tokio::test]
async fn test_twin_not_loaded_on_telemetry() {
    let runtime = Runtime::new(RuntimeConfig {
        eviction_timeout: Duration::from_millis(10),
        ..RuntimeConfig::default()
    });

    // Create twin
    let twin_id = runtime.create_twin("LazyTwin").await.unwrap();

    // Wait for it to become inactive
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Evict it
    let evicted = runtime.evict_inactive().await.unwrap();
    assert_eq!(evicted, 1);
    assert_eq!(runtime.stats().await.active_twins, 0);

    // Send telemetry - should NOT load the twin
    runtime
        .update_telemetry(twin_id, vec![("value".to_string(), 100.0)])
        .await
        .unwrap();

    // Twin should still not be loaded
    assert_eq!(runtime.stats().await.active_twins, 0);

    // But the event should be recorded
    let stats = runtime.stats().await;
    assert_eq!(stats.total_events, 2); // create + telemetry

    // When we access it, it should have the telemetry
    let active = runtime.get_twin(twin_id).await.unwrap();
    {
        let mut twin = active.twin.write().await;
        assert_eq!(twin.send(&msg!(value)).unwrap(), Value::from(100.0));
        drop(twin); // Explicitly drop the lock
    }
}

#[tokio::test]
async fn test_snapshot_and_restore() {
    let runtime = Runtime::new(RuntimeConfig::default());

    // Create twin with state
    let twin_id = runtime.create_twin("SnapshotTest").await.unwrap();
    runtime
        .update_telemetry(
            twin_id,
            vec![
                ("a".to_string(), 1.0),
                ("b".to_string(), 2.0),
                ("c".to_string(), 3.0),
            ],
        )
        .await
        .unwrap();

    // Create snapshot
    runtime.snapshot_twin(twin_id).await.unwrap();

    // Update more
    runtime
        .update_telemetry(twin_id, vec![("d".to_string(), 4.0)])
        .await
        .unwrap();

    // Evict and reload - should use snapshot + replay only recent events
    runtime.evict_inactive().await.unwrap();

    let active = runtime.get_twin(twin_id).await.unwrap();
    {
        let mut twin = active.twin.write().await;
        // Should have all properties
        assert_eq!(twin.send(&msg!(a)).unwrap(), Value::from(1.0));
        assert_eq!(twin.send(&msg!(b)).unwrap(), Value::from(2.0));
        assert_eq!(twin.send(&msg!(c)).unwrap(), Value::from(3.0));
        assert_eq!(twin.send(&msg!(d)).unwrap(), Value::from(4.0));
        drop(twin); // Explicitly drop the lock
    }
}

#[tokio::test]
async fn test_concurrent_access() {
    let runtime = Arc::new(Runtime::new(RuntimeConfig::default()));
    let twin_id = runtime.create_twin("ConcurrentTwin").await.unwrap();

    // Spawn multiple tasks that update the twin
    let mut handles = vec![];

    for i in 0..10 {
        let rt = runtime.clone();
        let id = twin_id;

        let handle = tokio::spawn(async move {
            rt.update_telemetry(id, vec![(format!("value_{i}"), f64::from(i))])
                .await
                .unwrap();
        });

        handles.push(handle);
    }

    // Wait for all updates
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all updates were applied
    let active = runtime.get_twin(twin_id).await.unwrap();
    {
        let mut twin = active.twin.write().await;
        for i in 0..10 {
            let value = twin
                .send(&Message::GetProperty(format!("value_{i}")))
                .unwrap();
            assert_eq!(value, Value::from(f64::from(i)));
        }
        drop(twin); // Explicitly drop the lock
    }
}

#[tokio::test]
async fn test_error_handling() {
    let runtime = Runtime::new(RuntimeConfig::default());

    // Try to get non-existent twin
    let fake_id = TwinId::new();
    let result = runtime.get_twin(fake_id).await;
    assert!(result.is_err());
    assert!(result.is_err());
    // Don't check the error message as it requires Debug trait
}
