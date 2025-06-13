//! Message dispatch system - the heart of our Smalltalk-inspired architecture
//!
//! Messages are pre-compiled for performance while maintaining flexibility.

use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Pre-compiled message for fast dispatch
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Message {
    /// Get property value: `twin temperature`
    GetProperty(String),

    /// Set property value: `twin temperature: 25.0`
    SetProperty(String, Value),

    /// Update multiple properties (telemetry)
    UpdateProperties(Vec<(String, Value)>),

    /// Send custom message with arguments
    Send {
        selector: String,
        args: Vec<Value>,
    },

    /// Twin lifecycle
    Clone,
    Initialize,
    Destroy,

    /// Inspection
    GetClass,
    GetAllProperties,
    RespondsTo(String),
}

impl Message {
    /// Parse a simple message from string (for REPL/debugging)
    /// This is NOT used in hot paths - only for interactive use
    pub fn parse(input: &str) -> Result<Self, String> {
        let parts: Vec<&str> = input.split_whitespace().collect();

        match parts.as_slice() {
            // Special messages first
            ["clone"] => Ok(Message::Clone),
            ["class"] => Ok(Message::GetClass),
            ["allProperties"] => Ok(Message::GetAllProperties),
            ["respondsTo:", selector] => Ok(Message::RespondsTo(selector.to_string())),

            // Property setter: "temperature: 25.0"
            [prop, ":", value] => {
                let prop_name = prop.trim_end_matches(':');
                let val = parse_value(value)?;
                Ok(Message::SetProperty(prop_name.to_string(), val))
            }

            // Property getter: "temperature" (must be last single-element pattern)
            [prop] => Ok(Message::GetProperty(prop.to_string())),

            // General message send
            _ => {
                if parts.is_empty() {
                    return Err("Empty message".to_string());
                }

                // Check if first part ends with colon (keyword message)
                if parts[0].ends_with(':') && parts.len() > 1 {
                    // Keyword message like "temperature: 25.0"
                    let prop_name = parts[0].trim_end_matches(':');
                    let val = parse_value(parts[1])?;
                    Ok(Message::SetProperty(prop_name.to_string(), val))
                } else if parts.len() > 1 && parts[1] == ":" {
                    // Simple keyword message with separate colon
                    let selector = format!("{}:", parts[0]);
                    let args = parts[2..]
                        .iter()
                        .map(|&s| parse_value(s))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(Message::Send { selector, args })
                } else {
                    // Unary message
                    Ok(Message::Send {
                        selector: parts[0].to_string(),
                        args: vec![],
                    })
                }
            }
        }
    }

    /// Get the selector (method name) for this message
    pub fn selector(&self) -> &str {
        match self {
            Message::GetProperty(p) => p,
            Message::SetProperty(p, _) => p,
            Message::UpdateProperties(_) => "updateProperties:",
            Message::Send { selector, .. } => selector,
            Message::Clone => "clone",
            Message::Initialize => "initialize",
            Message::Destroy => "destroy",
            Message::GetClass => "class",
            Message::GetAllProperties => "allProperties",
            Message::RespondsTo(_) => "respondsTo:",
        }
    }

    /// Get the number of arguments
    pub fn arg_count(&self) -> usize {
        match self {
            Message::GetProperty(_) => 0,
            Message::SetProperty(_, _) => 1,
            Message::UpdateProperties(props) => props.len(),
            Message::Send { args, .. } => args.len(),
            Message::Clone => 0,
            Message::Initialize => 0,
            Message::Destroy => 0,
            Message::GetClass => 0,
            Message::GetAllProperties => 0,
            Message::RespondsTo(_) => 1,
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::GetProperty(p) => write!(f, "{}", p),
            Message::SetProperty(p, v) => write!(f, "{}: {}", p, v),
            Message::UpdateProperties(props) => {
                write!(f, "updateProperties: [")?;
                for (i, (k, v)) in props.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "]")
            }
            Message::Send { selector, args } => {
                write!(f, "{}", selector)?;
                if !args.is_empty() {
                    write!(f, " ")?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, " ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                }
                Ok(())
            }
            Message::Clone => write!(f, "clone"),
            Message::Initialize => write!(f, "initialize"),
            Message::Destroy => write!(f, "destroy"),
            Message::GetClass => write!(f, "class"),
            Message::GetAllProperties => write!(f, "allProperties"),
            Message::RespondsTo(s) => write!(f, "respondsTo: {}", s),
        }
    }
}

/// Parse a simple value from string
fn parse_value(s: &str) -> Result<Value, String> {
    // Try parsing as number
    if let Ok(i) = s.parse::<i64>() {
        return Ok(Value::Integer(i));
    }
    if let Ok(f) = s.parse::<f64>() {
        return Ok(Value::Float(f.into()));
    }

    // Boolean
    match s {
        "true" => return Ok(Value::Boolean(true)),
        "false" => return Ok(Value::Boolean(false)),
        "nil" => return Ok(Value::Nil),
        _ => {}
    }

    // Symbol
    if let Some(stripped) = s.strip_prefix('#') {
        return Ok(Value::Symbol(stripped.to_string()));
    }

    // String (quoted)
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        return Ok(Value::String(s[1..s.len() - 1].to_string()));
    }

    // Default to string
    Ok(Value::String(s.to_string()))
}

/// Macro for compile-time message creation (zero overhead)
#[macro_export]
macro_rules! msg {
    // Special cases must come first to match before generic patterns

    // Clone: msg!(clone)
    (clone) => {
        $crate::message::Message::Clone
    };

    // Class: msg!(class)
    (class) => {
        $crate::message::Message::GetClass
    };

    // All properties: msg!(allProperties)
    (allProperties) => {
        $crate::message::Message::GetAllProperties
    };

    // Property setter: msg!(temperature: 25.0)
    ($prop:ident : $value:expr) => {
        $crate::message::Message::SetProperty(
            stringify!($prop).to_string(),
            $crate::value::Value::from($value),
        )
    };

    // Property getter: msg!(temperature) - must be last
    ($prop:ident) => {
        $crate::message::Message::GetProperty(stringify!($prop).to_string())
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_parse() {
        assert_eq!(
            Message::parse("temperature").unwrap(),
            Message::GetProperty("temperature".to_string())
        );

        assert_eq!(
            Message::parse("temperature: 25.0").unwrap(),
            Message::SetProperty("temperature".to_string(), Value::from(25.0))
        );

        assert_eq!(Message::parse("clone").unwrap(), Message::Clone);
    }

    #[test]
    fn test_message_macro() {
        assert_eq!(
            msg!(temperature),
            Message::GetProperty("temperature".to_string())
        );

        assert_eq!(
            msg!(temperature: 25.0),
            Message::SetProperty("temperature".to_string(), Value::from(25.0))
        );
    }
}
