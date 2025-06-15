//! Prototype-based programming using Rust trait objects
//! 
//! Explores using trait objects for dynamic behavior and cloning

use std::any::Any;
use std::collections::HashMap;
use dyn_clone::DynClone;

// Core trait that all twins must implement
trait Twin: DynClone + Send + Sync {
    fn get_id(&self) -> &str;
    fn get_class(&self) -> &str;
    
    // Dynamic property access
    fn get_property(&self, name: &str) -> Option<Box<dyn Any>>;
    fn set_property(&mut self, name: String, value: Box<dyn Any>);
    
    // Message handling
    fn handle_message(&mut self, selector: &str, args: &[Box<dyn Any>]) -> Result<Box<dyn Any>, String>;
    
    // Telemetry update
    fn update_telemetry(&mut self, data: &HashMap<String, f64>);
    
    // Clone with modifications
    fn clone_with_changes(&self, changes: HashMap<String, Box<dyn Any>>) -> Box<dyn Twin>;
}

dyn_clone::clone_trait_object!(Twin);

// Concrete implementation for a temperature sensor
#[derive(Clone)]
struct TemperatureSensor {
    id: String,
    temperature: f64,
    threshold: f64,
    alert_state: bool,
    custom_properties: HashMap<String, String>, // Simplified for the experiment
}

impl TemperatureSensor {
    fn new(id: String) -> Self {
        Self {
            id,
            temperature: 20.0,
            threshold: 30.0,
            alert_state: false,
            custom_properties: HashMap::new(),
        }
    }
}

impl Twin for TemperatureSensor {
    fn get_id(&self) -> &str {
        &self.id
    }
    
    fn get_class(&self) -> &str {
        "TemperatureSensor"
    }
    
    fn get_property(&self, name: &str) -> Option<Box<dyn Any>> {
        match name {
            "temperature" => Some(Box::new(self.temperature)),
            "threshold" => Some(Box::new(self.threshold)),
            "alert_state" => Some(Box::new(self.alert_state)),
            _ => None,
        }
    }
    
    fn set_property(&mut self, name: String, value: Box<dyn Any>) {
        match name.as_str() {
            "temperature" => {
                if let Ok(v) = value.downcast::<f64>() {
                    self.temperature = *v;
                }
            }
            "threshold" => {
                if let Ok(v) = value.downcast::<f64>() {
                    self.threshold = *v;
                }
            }
            _ => {
                // Store unknown properties dynamically
                // Note: This is simplified - real impl would need Send+Sync
            }
        }
    }
    
    fn handle_message(&mut self, selector: &str, _args: &[Box<dyn Any>]) -> Result<Box<dyn Any>, String> {
        match selector {
            "check_alert" => {
                let alert = self.temperature > self.threshold;
                self.alert_state = alert;
                Ok(Box::new(alert))
            }
            "reset" => {
                self.temperature = 20.0;
                self.alert_state = false;
                Ok(Box::new(()))
            }
            _ => Err(format!("Unknown message: {selector}")),
        }
    }
    
    fn update_telemetry(&mut self, data: &HashMap<String, f64>) {
        if let Some(&temp) = data.get("temperature") {
            self.temperature = temp;
            self.handle_message("check_alert", &[]).ok();
        }
    }
    
    fn clone_with_changes(&self, changes: HashMap<String, Box<dyn Any>>) -> Box<dyn Twin> {
        let mut cloned = self.clone();
        for (name, value) in changes {
            cloned.set_property(name, value);
        }
        Box::new(cloned)
    }
}

// Prototype registry for creating new instances
struct PrototypeRegistry {
    prototypes: HashMap<String, Box<dyn Twin>>,
}

impl PrototypeRegistry {
    fn new() -> Self {
        Self {
            prototypes: HashMap::new(),
        }
    }
    
    fn register(&mut self, name: String, prototype: Box<dyn Twin>) {
        self.prototypes.insert(name, prototype);
    }
    
    fn create_from(&self, prototype_name: &str, id: String, changes: HashMap<String, Box<dyn Any>>) -> Option<Box<dyn Twin>> {
        self.prototypes.get(prototype_name)
            .map(|proto| {
                let mut cloned = proto.clone_with_changes(changes);
                cloned.set_property("id".to_string(), Box::new(id));
                cloned
            })
    }
}

fn main() {
    println!("=== Trait Object Prototype Pattern ===\n");
    
    // Create prototype registry
    let mut registry = PrototypeRegistry::new();
    
    // Register a sensor prototype
    let sensor_proto = Box::new(TemperatureSensor::new("prototype".to_string()));
    registry.register("TemperatureSensor".to_string(), sensor_proto);
    
    // Create instances from prototype
    let mut sensor1 = registry.create_from(
        "TemperatureSensor",
        "sensor_1".to_string(),
        HashMap::new()
    ).unwrap();
    
    let mut changes = HashMap::new();
    changes.insert("threshold".to_string(), Box::new(25.0) as Box<dyn Any>);
    let mut sensor2 = registry.create_from(
        "TemperatureSensor",
        "sensor_2".to_string(),
        changes
    ).unwrap();
    
    println!("Sensor 1 threshold: {:?}", sensor1.get_property("threshold"));
    println!("Sensor 2 threshold: {:?}", sensor2.get_property("threshold"));
    
    // Update telemetry
    let mut telemetry = HashMap::new();
    telemetry.insert("temperature".to_string(), 27.0);
    
    sensor1.update_telemetry(&telemetry);
    sensor2.update_telemetry(&telemetry);
    
    println!("\nAfter telemetry update (27.0°C):");
    println!("Sensor 1 alert: {:?}", sensor1.get_property("alert_state"));
    println!("Sensor 2 alert: {:?}", sensor2.get_property("alert_state"));
    
    // Performance test
    use std::time::Instant;
    println!("\n=== Performance Test ===");
    
    let start = Instant::now();
    for i in 0..10000 {
        let mut sensor = registry.create_from(
            "TemperatureSensor",
            format!("perf_sensor_{i}"),
            HashMap::new()
        ).unwrap();
        
        let mut telemetry = HashMap::new();
        telemetry.insert("temperature".to_string(), 25.0);
        sensor.update_telemetry(&telemetry);
    }
    let elapsed = start.elapsed();
    
    println!("Created and updated 10k sensors in {:?}", elapsed);
    println!("Average per sensor: {:?}", elapsed / 10_000);
}