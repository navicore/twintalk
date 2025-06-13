//! Tests for twin functionality

use std::collections::BTreeMap;
use twintalk_core::{msg, Message, Twin, TwinId, Value};

#[test]
fn test_twin_creation() {
    let twin = Twin::new("TemperatureSensor");
    assert_eq!(twin.class_name(), "TemperatureSensor");
    assert!(twin.id() != TwinId::new()); // ID should be unique
}

#[test]
fn test_property_management() {
    let mut twin = Twin::new("Sensor");

    // Set property
    let result = twin.send(&msg!(temperature: 25.0));
    assert!(result.is_ok());

    // Get property
    let value = twin.send(&msg!(temperature)).unwrap();
    assert_eq!(value, Value::from(25.0));

    // Get non-existent property returns nil
    let value = twin.send(&msg!(nonexistent)).unwrap();
    assert_eq!(value, Value::Nil);
}

#[test]
fn test_bulk_property_update() {
    let mut twin = Twin::new("Sensor");

    // Update multiple properties at once
    let updates = vec![
        ("temperature".to_string(), Value::from(22.5)),
        ("humidity".to_string(), Value::from(45.0)),
        ("pressure".to_string(), Value::from(1013.25)),
    ];

    twin.send(&Message::UpdateProperties(updates)).unwrap();

    // Verify all properties were set
    assert_eq!(twin.send(&msg!(temperature)).unwrap(), Value::from(22.5));
    assert_eq!(twin.send(&msg!(humidity)).unwrap(), Value::from(45.0));
    assert_eq!(twin.send(&msg!(pressure)).unwrap(), Value::from(1013.25));
}

#[test]
fn test_twin_cloning() {
    let mut original = Twin::new("Sensor");
    original.send(&msg!(temperature: 20.0)).unwrap();
    original.send(&msg!(threshold: 30.0)).unwrap();

    let mut cloned = original.clone_twin();

    // Cloned twin should have different ID
    assert_ne!(original.id(), cloned.id());

    // Cloned twin should have same class
    assert_eq!(original.class_name(), cloned.class_name());

    // State should be independent - modifying original doesn't affect clone
    original.send(&msg!(temperature: 25.0)).unwrap();
    assert_eq!(cloned.send(&msg!(temperature)).unwrap(), Value::from(20.0));
}

#[test]
fn test_builtin_messages() {
    let mut twin = Twin::new("TestTwin");
    twin.send(&msg!(foo: "bar")).unwrap();

    // Get class
    let class = twin.send(&msg!(class)).unwrap();
    assert_eq!(class, Value::String("TestTwin".to_string()));

    // Get all properties
    let props = twin.send(&msg!(allProperties)).unwrap();
    match props {
        Value::Map(map) => {
            assert_eq!(map.get("foo"), Some(&Value::String("bar".to_string())));
        }
        _ => panic!("Expected map"),
    }

    // RespondsTo
    let responds = twin
        .send(&Message::RespondsTo("class".to_string()))
        .unwrap();
    assert_eq!(responds, Value::Boolean(true));

    let responds = twin
        .send(&Message::RespondsTo("unknownMethod".to_string()))
        .unwrap();
    assert_eq!(responds, Value::Boolean(false));
}

#[test]
fn test_telemetry_update() {
    let mut twin = Twin::new("Sensor");

    let mut telemetry = BTreeMap::new();
    telemetry.insert("temperature".to_string(), 25.5);
    telemetry.insert("humidity".to_string(), 60.0);

    twin.update_telemetry(telemetry).unwrap();

    assert_eq!(twin.send(&msg!(temperature)).unwrap(), Value::from(25.5));
    assert_eq!(twin.send(&msg!(humidity)).unwrap(), Value::from(60.0));
}

#[test]
fn test_custom_message_check_alert() {
    let mut twin = Twin::new("Sensor");
    twin.send(&msg!(temperature: 25.0)).unwrap();
    twin.send(&msg!(threshold: 30.0)).unwrap();

    // Check alert when temperature is below threshold
    let alert = twin
        .send(&Message::Send {
            selector: "checkAlert".to_string(),
            args: vec![],
        })
        .unwrap();
    assert_eq!(alert, Value::Boolean(false));

    // Update temperature above threshold
    twin.send(&msg!(temperature: 35.0)).unwrap();

    // Check alert again
    let alert = twin
        .send(&Message::Send {
            selector: "checkAlert".to_string(),
            args: vec![],
        })
        .unwrap();
    assert_eq!(alert, Value::Boolean(true));

    // Verify alert property was set
    assert_eq!(twin.send(&msg!(alert)).unwrap(), Value::Boolean(true));
}

#[test]
fn test_error_handling() {
    let mut twin = Twin::new("Sensor");

    // Unknown message should return error
    let result = twin.send(&Message::Send {
        selector: "unknownMessage".to_string(),
        args: vec![],
    });

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not understand"));
}
