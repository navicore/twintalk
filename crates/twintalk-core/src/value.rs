//! Value type for twin properties and message arguments
//!
//! Supports the minimal set needed for digital twins without
//! full Smalltalk object complexity.

use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Core value type for twin state and messages
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(tag = "type", content = "value")]
pub enum Value {
    /// Nil/null value
    #[default]
    Nil,

    /// Boolean value
    Boolean(bool),

    /// Integer number
    Integer(i64),

    /// Floating point number
    Float(OrderedFloat<f64>),

    /// UTF-8 string
    String(String),

    /// Symbol (interned string)
    Symbol(String),

    /// Array of values
    Array(Vec<Value>),

    /// Key-value map
    Map(BTreeMap<String, Value>),

    /// Binary data
    Bytes(Vec<u8>),
}

impl Value {
    /// Convert to boolean if possible
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Boolean(b) => Some(*b),
            Value::Nil => Some(false),
            _ => None,
        }
    }

    /// Convert to integer if possible
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            Value::Float(f) => Some(f.into_inner() as i64),
            _ => None,
        }
    }

    /// Convert to float if possible
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(f.into_inner()),
            Value::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Convert to string if possible
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            Value::Symbol(s) => Some(s),
            _ => None,
        }
    }

    /// Check if value is truthy (Smalltalk semantics)
    pub fn is_truthy(&self) -> bool {
        !matches!(self, Value::Nil | Value::Boolean(false))
    }

    /// Type name for inspection
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "Nil",
            Value::Boolean(_) => "Boolean",
            Value::Integer(_) => "Integer",
            Value::Float(_) => "Float",
            Value::String(_) => "String",
            Value::Symbol(_) => "Symbol",
            Value::Array(_) => "Array",
            Value::Map(_) => "Map",
            Value::Bytes(_) => "Bytes",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "{}", s),
            Value::Symbol(s) => write!(f, "#{}", s),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Map(map) => {
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Bytes(b) => write!(f, "<{} bytes>", b.len()),
        }
    }
}

// Conversions from Rust types
impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Boolean(b)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Integer(i as i64)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Integer(i)
    }
}

impl From<f32> for Value {
    fn from(f: f32) -> Self {
        Value::Float(OrderedFloat(f as f64))
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(OrderedFloat(f))
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(vec: Vec<T>) -> Self {
        Value::Array(vec.into_iter().map(Into::into).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_conversions() {
        assert_eq!(Value::from(42).as_i64(), Some(42));
        assert_eq!(Value::from(3.5).as_f64(), Some(3.5));
        assert_eq!(Value::from("hello").as_str(), Some("hello"));
        assert_eq!(Value::from(true).as_bool(), Some(true));
    }

    #[test]
    fn test_truthy() {
        assert!(Value::from(true).is_truthy());
        assert!(Value::from(42).is_truthy());
        assert!(Value::from("hello").is_truthy());
        assert!(!Value::Nil.is_truthy());
        assert!(!Value::from(false).is_truthy());
    }
}
