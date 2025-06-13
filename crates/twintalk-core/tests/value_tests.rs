//! Tests for the Value type system

use std::collections::BTreeMap;
use twintalk_core::Value;

#[test]
fn test_value_conversions() {
    // From primitives
    assert_eq!(Value::from(42), Value::Integer(42));
    assert_eq!(Value::from(3.5), Value::Float(3.5.into()));
    assert_eq!(Value::from("hello"), Value::String("hello".to_string()));
    assert_eq!(Value::from(true), Value::Boolean(true));

    // To primitives
    assert_eq!(Value::Integer(42).as_i64(), Some(42));
    assert_eq!(Value::Float(3.5.into()).as_f64(), Some(3.5));
    assert_eq!(Value::String("hello".to_string()).as_str(), Some("hello"));
    assert_eq!(Value::Boolean(true).as_bool(), Some(true));

    // Cross-numeric conversions
    assert_eq!(Value::Integer(42).as_f64(), Some(42.0));
    assert_eq!(Value::Float(42.0.into()).as_i64(), Some(42));
}

#[test]
fn test_value_truthy() {
    // Truthy values
    assert!(Value::Boolean(true).is_truthy());
    assert!(Value::Integer(1).is_truthy());
    assert!(Value::Float(1.0.into()).is_truthy());
    assert!(Value::String("hello".to_string()).is_truthy());
    assert!(Value::Array(vec![]).is_truthy());

    // Falsy values
    assert!(!Value::Nil.is_truthy());
    assert!(!Value::Boolean(false).is_truthy());
}

#[test]
fn test_value_equality() {
    // Same values should be equal
    assert_eq!(Value::Integer(42), Value::Integer(42));
    assert_eq!(Value::Float(3.5.into()), Value::Float(3.5.into()));
    assert_eq!(
        Value::String("hello".to_string()),
        Value::String("hello".to_string())
    );

    // Different values should not be equal
    assert_ne!(Value::Integer(42), Value::Integer(43));
    assert_ne!(Value::Integer(42), Value::Float(42.0.into()));
}

#[test]
fn test_value_collections() {
    // Arrays
    let arr = Value::Array(vec![Value::Integer(1), Value::Integer(2)]);
    match arr {
        Value::Array(values) => {
            assert_eq!(values.len(), 2);
            assert_eq!(values[0], Value::Integer(1));
        }
        _ => panic!("Expected array"),
    }

    // Maps
    let mut map = BTreeMap::new();
    map.insert("key".to_string(), Value::String("value".to_string()));
    let map_val = Value::Map(map.clone());

    match map_val {
        Value::Map(m) => {
            assert_eq!(m.get("key"), Some(&Value::String("value".to_string())));
        }
        _ => panic!("Expected map"),
    }
}

#[test]
fn test_value_display() {
    assert_eq!(Value::Nil.to_string(), "nil");
    assert_eq!(Value::Boolean(true).to_string(), "true");
    assert_eq!(Value::Integer(42).to_string(), "42");
    assert_eq!(Value::Float(3.5.into()).to_string(), "3.5");
    assert_eq!(Value::String("hello".to_string()).to_string(), "hello");
    assert_eq!(Value::Symbol("foo".to_string()).to_string(), "#foo");
}
