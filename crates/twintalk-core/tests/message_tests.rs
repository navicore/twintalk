//! Tests for the message passing system

use twintalk_core::{msg, Message, Value};

#[test]
fn test_message_macro() {
    // Property getter
    let msg = msg!(temperature);
    assert_eq!(msg, Message::GetProperty("temperature".to_string()));

    // Property setter with different types
    let msg = msg!(temperature: 25.0);
    assert_eq!(
        msg,
        Message::SetProperty("temperature".to_string(), Value::from(25.0))
    );

    let msg = msg!(enabled: true);
    assert_eq!(
        msg,
        Message::SetProperty("enabled".to_string(), Value::from(true))
    );

    // Special messages
    let msg = msg!(clone);
    assert_eq!(msg, Message::Clone);

    let msg = msg!(class);
    assert_eq!(msg, Message::GetClass);
}

#[test]
fn test_message_parsing() {
    // Simple property getter
    let msg = Message::parse("temperature").unwrap();
    assert_eq!(msg, Message::GetProperty("temperature".to_string()));

    // Property setter
    let msg = Message::parse("temperature: 25.0").unwrap();
    assert_eq!(
        msg,
        Message::SetProperty("temperature".to_string(), Value::from(25.0))
    );

    // Special messages
    assert_eq!(Message::parse("clone").unwrap(), Message::Clone);
    assert_eq!(Message::parse("class").unwrap(), Message::GetClass);
    assert_eq!(
        Message::parse("allProperties").unwrap(),
        Message::GetAllProperties
    );

    // RespondsTo
    let msg = Message::parse("respondsTo: update").unwrap();
    assert_eq!(msg, Message::RespondsTo("update".to_string()));
}

#[test]
fn test_message_selector() {
    assert_eq!(Message::GetProperty("temp".to_string()).selector(), "temp");
    assert_eq!(Message::Clone.selector(), "clone");
    assert_eq!(Message::GetClass.selector(), "class");
}

#[test]
fn test_message_display() {
    let msg = Message::GetProperty("temperature".to_string());
    assert_eq!(msg.to_string(), "temperature");

    let msg = Message::SetProperty("temperature".to_string(), Value::from(25.0));
    assert_eq!(msg.to_string(), "temperature: 25");

    assert_eq!(Message::Clone.to_string(), "clone");
}
