//! API usage examples that serve as both documentation and regression tests
//!
//! These tests demonstrate common patterns for using the TwinTalk core API.

use std::sync::Arc;
use std::time::Duration;
use twintalk_core::{msg, Message, Runtime, RuntimeConfig, Twin, Value};

/// Example: Basic digital twin for IoT sensor
#[tokio::test]
async fn example_iot_sensor() {
    // Create runtime with default config
    let runtime = Runtime::new(RuntimeConfig::default());

    // Create a temperature sensor twin
    let sensor_id = runtime.create_twin("TemperatureSensor").await.unwrap();

    // Send initial configuration
    runtime
        .update_telemetry(
            sensor_id,
            vec![
                ("location".to_string(), 1.0), // Using float for simplicity
                ("threshold".to_string(), 25.0),
                ("alert_enabled".to_string(), 1.0), // 1.0 = true
            ],
        )
        .await
        .unwrap();

    // Simulate telemetry updates
    for temp in [20.0, 22.0, 26.0, 24.0] {
        runtime
            .update_telemetry(sensor_id, vec![("temperature".to_string(), temp)])
            .await
            .unwrap();

        // In real usage, you might check alerts here
    }

    // Query final state
    let active = runtime.get_twin(sensor_id).await.unwrap();
    let mut twin = active.twin.write().await;

    let temp = twin.send(&msg!(temperature)).unwrap();
    assert_eq!(temp, Value::from(24.0));
}

/// Example: Prototype-based twin customization
#[tokio::test]
async fn example_prototype_cloning() {
    // Direct twin API (without runtime) for prototyping
    let mut base_sensor = Twin::new("Sensor");

    // Configure base sensor
    base_sensor.send(&msg!(unit: "celsius")).unwrap();
    base_sensor.send(&msg!(interval: 60.0)).unwrap(); // seconds
    base_sensor.send(&msg!(precision: 0.1)).unwrap();

    // Clone and customize for different locations
    let mut outdoor_sensor = base_sensor.clone_twin();
    outdoor_sensor.send(&msg!(location: "outdoor")).unwrap();
    outdoor_sensor.send(&msg!(threshold: 35.0)).unwrap();

    let mut indoor_sensor = base_sensor.clone_twin();
    indoor_sensor.send(&msg!(location: "indoor")).unwrap();
    indoor_sensor.send(&msg!(threshold: 25.0)).unwrap();

    // Verify customizations
    assert_eq!(
        outdoor_sensor.send(&msg!(threshold)).unwrap(),
        Value::from(35.0)
    );
    assert_eq!(
        indoor_sensor.send(&msg!(threshold)).unwrap(),
        Value::from(25.0)
    );

    // Both inherit base properties
    assert_eq!(
        outdoor_sensor.send(&msg!(unit)).unwrap(),
        Value::String("celsius".to_string())
    );
}

/// Example: Message passing patterns
#[tokio::test]
async fn example_message_patterns() {
    let mut twin = Twin::new("SmartDevice");

    // Property getter/setter pattern
    twin.send(&msg!(power: "on")).unwrap();
    let power = twin.send(&msg!(power)).unwrap();
    assert_eq!(power, Value::String("on".to_string()));

    // Bulk updates
    twin.send(&Message::UpdateProperties(vec![
        ("brightness".to_string(), Value::from(75.0)),
        ("color".to_string(), Value::String("white".to_string())),
        ("mode".to_string(), Value::String("auto".to_string())),
    ]))
    .unwrap();

    // Query all properties
    let all_props = twin.send(&msg!(allProperties)).unwrap();
    match all_props {
        Value::Map(props) => {
            assert_eq!(props.get("brightness"), Some(&Value::from(75.0)));
            assert_eq!(
                props.get("color"),
                Some(&Value::String("white".to_string()))
            );
        }
        _ => panic!("Expected map"),
    }

    // Check capabilities
    let can_alert = twin
        .send(&Message::RespondsTo("checkAlert".to_string()))
        .unwrap();
    assert_eq!(can_alert, Value::Boolean(true));
}

/// Example: Event sourcing and persistence
#[tokio::test]
async fn example_event_sourcing() {
    let runtime = Arc::new(Runtime::new(RuntimeConfig {
        eviction_timeout: Duration::from_secs(1),
        eviction_interval: Duration::from_secs(1),
        snapshot_on_eviction: true,
        max_active_twins: Some(100),
    }));

    // Create and configure twin
    let device_id = runtime.create_twin("SmartMeter").await.unwrap();

    // Simulate hourly readings
    for hour in 0..24 {
        runtime
            .update_telemetry(
                device_id,
                vec![
                    ("hour".to_string(), hour as f64),
                    ("consumption".to_string(), 1.5 + (hour as f64 * 0.1)),
                ],
            )
            .await
            .unwrap();
    }

    // Force eviction to test persistence
    runtime.evict_inactive().await.unwrap();

    // Twin should be reloaded from events
    let active = runtime.get_twin(device_id).await.unwrap();
    let mut twin = active.twin.write().await;

    // Should have last reading
    assert_eq!(twin.send(&msg!(hour)).unwrap(), Value::from(23.0));
    // Use approximate comparison for floats due to precision
    let consumption = twin.send(&msg!(consumption)).unwrap();
    if let Value::Float(f) = consumption {
        assert!((f.into_inner() - 3.8).abs() < 0.0001);
    } else {
        panic!("Expected float value");
    }
}

/// Example: Custom message handlers
#[tokio::test]
async fn example_custom_messages() {
    let mut twin = Twin::new("ThermostatController");

    // Set up state
    twin.send(&msg!(temperature: 22.0)).unwrap();
    twin.send(&msg!(setpoint: 20.0)).unwrap();
    twin.send(&msg!(mode: "heating")).unwrap();

    // Custom message: calculate if heating needed
    // In a real system, you'd extend the twin with custom handlers
    let temp_val = twin.send(&msg!(temperature)).unwrap();
    let temp = temp_val.as_f64().unwrap();
    let setpoint_val = twin.send(&msg!(setpoint)).unwrap();
    let setpoint = setpoint_val.as_f64().unwrap();
    let mode_val = twin.send(&msg!(mode)).unwrap();
    let mode = mode_val.as_str().unwrap();

    let heating_needed = mode == "heating" && temp < setpoint;
    twin.send(&msg!(heating_active: heating_needed)).unwrap();

    assert_eq!(
        twin.send(&msg!(heating_active)).unwrap(),
        Value::Boolean(false) // 22 > 20, so no heating needed
    );
}

/// Example: Working with twin metadata
#[tokio::test]
async fn example_twin_metadata() {
    let runtime = Runtime::new(RuntimeConfig::default());

    // Create twins with meaningful class names
    let sensor_id = runtime.create_twin("TemperatureSensor").await.unwrap();
    let actuator_id = runtime.create_twin("HeaterActuator").await.unwrap();
    let controller_id = runtime.create_twin("ClimateController").await.unwrap();

    // Use class information for routing/filtering
    let twins = vec![
        (sensor_id, runtime.get_twin(sensor_id).await.unwrap()),
        (actuator_id, runtime.get_twin(actuator_id).await.unwrap()),
        (
            controller_id,
            runtime.get_twin(controller_id).await.unwrap(),
        ),
    ];

    for (id, active) in twins {
        let mut twin = active.twin.write().await;
        let class = twin.send(&msg!(class)).unwrap();

        match class.as_str() {
            Some("TemperatureSensor") => {
                // Configure sensor-specific properties
                drop(twin);
                runtime
                    .update_telemetry(id, vec![("sample_rate".to_string(), 1.0)])
                    .await
                    .unwrap();
            }
            Some("HeaterActuator") => {
                // Configure actuator-specific properties
                drop(twin);
                runtime
                    .update_telemetry(id, vec![("max_power".to_string(), 2000.0)])
                    .await
                    .unwrap();
            }
            _ => {}
        }
    }
}
