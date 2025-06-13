//! Experiment using Rhai as an alternative scripting language
//! 
//! Tests if we could use Rhai with Smalltalk-like syntax sugar

use rhai::{Engine, Dynamic};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone)]
struct Twin {
    id: String,
    state: Arc<Mutex<HashMap<String, Dynamic>>>,
}

impl Twin {
    fn new(id: String) -> Self {
        Self {
            id,
            state: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    fn get_state(&mut self, key: &str) -> Dynamic {
        self.state.lock().unwrap()
            .get(key)
            .cloned()
            .unwrap_or(Dynamic::UNIT)
    }
    
    fn set_state(&mut self, key: &str, value: Dynamic) {
        self.state.lock().unwrap().insert(key.to_string(), value);
    }
    
    fn clone_twin(&mut self) -> Twin {
        let new_state = self.state.lock().unwrap().clone();
        Twin {
            id: format!("{}_clone", self.id),
            state: Arc::new(Mutex::new(new_state)),
        }
    }
}

fn main() {
    println!("=== Rhai Scripting Experiment ===\n");
    
    let mut engine = Engine::new();
    
    // Register Twin type
    engine.register_type_with_name::<Twin>("Twin")
        .register_fn("new_twin", Twin::new)
        .register_fn("get", Twin::get_state)
        .register_fn("set", Twin::set_state)
        .register_fn("clone", Twin::clone_twin);
    
    // Define a temperature sensor twin in Rhai
    let sensor_script = r#"
        // Create a temperature sensor twin
        let sensor = new_twin("temp_sensor_1");
        sensor.set("temperature", 20.0);
        sensor.set("threshold", 30.0);
        sensor.set("alert", false);
        
        // Update telemetry function
        fn update_telemetry(twin, reading) {
            twin.set("temperature", reading);
            
            if reading > twin.get("threshold") {
                twin.set("alert", true);
                print("ALERT: Temperature " + reading + " exceeds threshold!");
            } else {
                twin.set("alert", false);
            }
        }
        
        // Simulate telemetry updates
        update_telemetry(sensor, 25.0);
        update_telemetry(sensor, 35.0);
        
        // Clone and customize
        let custom_sensor = sensor.clone();
        custom_sensor.set("threshold", 25.0);
        
        print("Original threshold: " + sensor.get("threshold"));
        print("Cloned threshold: " + custom_sensor.get("threshold"));
        
        sensor
    "#;
    
    let mut result = engine.eval::<Twin>(sensor_script).unwrap();
    println!("\nFinal sensor state:");
    println!("  Temperature: {:?}", result.get_state("temperature"));
    println!("  Alert: {:?}", result.get_state("alert"));
    
    // Performance test
    let perf_script = r#"
        let twin = new_twin("perf_test");
        twin.set("value", 0);
        
        for i in 0..10000 {
            let current = twin.get("value");
            twin.set("value", current + 1);
        }
        
        twin.get("value")
    "#;
    
    println!("\n=== Performance Test ===");
    let start = Instant::now();
    let result = engine.eval::<i64>(perf_script).unwrap();
    let elapsed = start.elapsed();
    
    println!("10k state updates took: {:?}", elapsed);
    println!("Final value: {}", result);
    println!("Average per update: {:?}", elapsed / 10_000);
    
    // Test with Smalltalk-ish syntax via preprocessing
    println!("\n=== Smalltalk-style Syntax ===");
    
    // We could preprocess this syntax:
    // sensor temperature: 25.0
    // Into this Rhai:
    // sensor.set("temperature", 25.0)
    
    let smalltalk_ish = r#"
        let sensor = new_twin("st_sensor");
        
        // Simulated Smalltalk-style message (after preprocessing)
        sensor.set("temperature", 25.0);
        sensor.set("threshold", 30.0);
        
        // Direct check instead of closure for simplicity
        let temp = sensor.get("temperature");
        let thresh = sensor.get("threshold");
        let is_alert = temp > thresh;
        
        sensor.set("alert", is_alert);
        sensor.get("alert")
    "#;
    
    let result = engine.eval::<bool>(smalltalk_ish).unwrap();
    println!("Alert status: {}", result);
}