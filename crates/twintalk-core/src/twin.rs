//! Digital Twin implementation with prototype-based programming
//!
//! Twins are the core entities that receive telemetry and respond to messages.

use crate::message::Message;
use crate::value::Value;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a twin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TwinId(pub Uuid);

impl TwinId {
    /// Create a new unique twin ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TwinId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TwinId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Twin state that can be persisted and restored
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinState {
    pub id: TwinId,
    pub class_name: String,
    pub properties: BTreeMap<String, Value>,
    pub parent_id: Option<TwinId>, // For prototype chain
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub is_hypothetical: bool, // Marks clones used for prediction
    #[serde(default)]
    pub simulation_time: Option<DateTime<Utc>>, // Virtual time for hypothetical twins
}

/// Active twin instance with behavior
pub struct Twin {
    pub state: TwinState,
}

impl Twin {
    /// Create a new twin with given class name
    pub fn new(class_name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            state: TwinState {
                id: TwinId::new(),
                class_name: class_name.into(),
                properties: BTreeMap::new(),
                parent_id: None,
                created_at: now,
                updated_at: now,
                is_hypothetical: false,
                simulation_time: None,
            },
        }
    }

    /// Create from existing state (for loading from persistence)
    pub fn from_state(state: TwinState) -> Self {
        Self { state }
    }

    /// Get the twin's ID
    pub fn id(&self) -> TwinId {
        self.state.id
    }

    /// Get the twin's class name
    pub fn class_name(&self) -> &str {
        &self.state.class_name
    }

    /// Get the current state (for persistence)
    pub fn state(&self) -> &TwinState {
        &self.state
    }

    /// Clone this twin (prototype-based)
    #[must_use]
    pub fn clone_twin(&self) -> Self {
        let mut new_state = self.state.clone();
        new_state.id = TwinId::new();
        new_state.parent_id = Some(self.state.id);
        new_state.created_at = Utc::now();
        new_state.updated_at = new_state.created_at;

        Self { state: new_state }
    }

    /// Clone this twin as hypothetical (for predictions/simulations)
    #[must_use]
    pub fn clone_hypothetical(&self) -> Self {
        let mut new_state = self.state.clone();
        new_state.id = TwinId::new();
        new_state.parent_id = Some(self.state.id);
        new_state.created_at = Utc::now();
        new_state.updated_at = new_state.created_at;
        new_state.is_hypothetical = true;
        new_state.simulation_time = Some(Utc::now());

        Self { state: new_state }
    }

    /// Check if this twin is hypothetical
    pub fn is_hypothetical(&self) -> bool {
        self.state.is_hypothetical
    }

    /// Get the simulation time for hypothetical twins
    pub fn simulation_time(&self) -> Option<DateTime<Utc>> {
        self.state.simulation_time
    }

    /// Set the simulation time (for hypothetical twins)
    pub fn set_simulation_time(&mut self, time: DateTime<Utc>) -> Result<()> {
        if !self.state.is_hypothetical {
            return Err(anyhow!("Cannot set simulation time on non-hypothetical twin"));
        }
        self.state.simulation_time = Some(time);
        Ok(())
    }

    /// Send a message to this twin
    pub fn send(&mut self, message: &Message) -> Result<Value> {
        self.state.updated_at = Utc::now();

        match message {
            Message::GetProperty(name) => Ok(self
                .state
                .properties
                .get(name)
                .cloned()
                .unwrap_or(Value::Nil)),

            Message::SetProperty(name, value) => {
                self.state.properties.insert(name.clone(), value.clone());
                Ok(Value::Nil)
            }

            Message::UpdateProperties(updates) => {
                for (name, value) in updates {
                    self.state.properties.insert(name.clone(), value.clone());
                }
                Ok(Value::Nil)
            }

            Message::Clone => {
                // Return the cloned twin's ID
                let cloned = self.clone_twin();
                Ok(Value::String(cloned.id().to_string()))
            }

            Message::GetClass => Ok(Value::String(self.state.class_name.clone())),

            Message::GetAllProperties => Ok(Value::Map(self.state.properties.clone())),

            Message::RespondsTo(selector) => {
                let responds = Self::responds_to_builtin(selector);
                Ok(Value::Boolean(responds))
            }

            Message::Send { selector, args } => self.handle_custom_message(selector, args),

            _ => Err(anyhow!("Unhandled message: {message:?}")),
        }
    }

    /// Update from telemetry data
    pub fn update_telemetry(&mut self, data: BTreeMap<String, f64>) -> Result<()> {
        let updates: Vec<(String, Value)> = data
            .into_iter()
            .map(|(k, v)| (k, Value::Float(v.into())))
            .collect();

        self.send(&Message::UpdateProperties(updates))?;
        Ok(())
    }

    /// Check if twin responds to built-in messages
    fn responds_to_builtin(selector: &str) -> bool {
        matches!(
            selector,
            "class" | "allProperties" | "clone" | "respondsTo:" | "checkAlert"
        )
    }

    /// Handle custom messages
    fn handle_custom_message(&mut self, selector: &str, _args: &[Value]) -> Result<Value> {
        match selector {
            "checkAlert" => {
                let temp = self
                    .state
                    .properties
                    .get("temperature")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let threshold = self
                    .state
                    .properties
                    .get("threshold")
                    .and_then(Value::as_f64)
                    .unwrap_or(30.0);

                let alert = temp > threshold;
                self.state
                    .properties
                    .insert("alert".to_string(), Value::Boolean(alert));
                Ok(Value::Boolean(alert))
            }
            _ => Err(anyhow!("Twin does not understand: {selector}")),
        }
    }
}

impl Clone for Twin {
    fn clone(&self) -> Self {
        self.clone_twin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg;

    #[test]
    fn test_twin_creation() {
        let twin = Twin::new("TemperatureSensor");
        assert_eq!(twin.class_name(), "TemperatureSensor");
    }

    #[test]
    fn test_property_access() {
        let mut twin = Twin::new("Sensor");

        // Set property
        twin.send(&msg!(temperature: 25.0)).unwrap();

        // Get property
        let temp = twin.send(&msg!(temperature)).unwrap();
        assert_eq!(temp, Value::from(25.0));
    }

    #[test]
    fn test_twin_cloning() {
        let mut original = Twin::new("Sensor");
        original.send(&msg!(temperature: 20.0)).unwrap();

        let cloned = original.clone_twin();
        assert_ne!(original.id(), cloned.id());
        assert_eq!(cloned.state.parent_id, Some(original.id()));
    }
}
