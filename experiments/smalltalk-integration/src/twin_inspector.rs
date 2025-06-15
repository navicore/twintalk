//! Runtime twin inspector with minimal Smalltalk syntax
//! 
//! Demonstrates practical runtime inspection without full parsing overhead

use std::collections::HashMap;
use std::io::{self, Write};

#[derive(Debug, Clone)]
enum Value {
    Nil,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Symbol(String),
}

struct TwinInspector {
    twins: HashMap<String, Twin>,
}

struct Twin {
    id: String,
    class_name: String,
    properties: HashMap<String, Value>,
}

impl TwinInspector {
    fn new() -> Self {
        Self {
            twins: HashMap::new(),
        }
    }
    
    fn create_test_twins(&mut self) {
        // Create some test twins
        for i in 0..5 {
            let mut twin = Twin {
                id: format!("sensor_{i}"),
                class_name: "TemperatureSensor".to_string(),
                properties: HashMap::new(),
            };
            
            twin.properties.insert("temperature".to_string(), Value::Float(20.0 + i as f64));
            twin.properties.insert("threshold".to_string(), Value::Float(30.0));
            twin.properties.insert("alert".to_string(), Value::Boolean(i > 2));
            
            self.twins.insert(twin.id.clone(), twin);
        }
    }
    
    // Simple command parser - no full Smalltalk needed
    fn execute_command(&mut self, input: &str) -> Result<String, String> {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        
        if parts.is_empty() {
            return Ok("".to_string());
        }
        
        match parts[0] {
            "list" => {
                let mut result = String::from("Twins:\n");
                for (id, twin) in &self.twins {
                    result.push_str(&format!("  {} ({})\n", id, twin.class_name));
                }
                Ok(result)
            }
            
            "inspect" => {
                if parts.len() < 2 {
                    return Err("Usage: inspect <twin_id>".to_string());
                }
                
                if let Some(twin) = self.twins.get(parts[1]) {
                    let mut result = format!("Twin: {}\n", twin.id);
                    result.push_str(&format!("Class: {}\n", twin.class_name));
                    result.push_str("Properties:\n");
                    
                    for (key, value) in &twin.properties {
                        result.push_str(&format!("  {key}: {value:?}\n"));
                    }
                    Ok(result)
                } else {
                    Err(format!("Twin '{}' not found", parts[1]))
                }
            }
            
            "get" => {
                // get sensor_1 temperature
                if parts.len() < 3 {
                    return Err("Usage: get <twin_id> <property>".to_string());
                }
                
                if let Some(twin) = self.twins.get(parts[1]) {
                    if let Some(value) = twin.properties.get(parts[2]) {
                        Ok(format!("{value:?}"))
                    } else {
                        Err(format!("Property '{}' not found", parts[2]))
                    }
                } else {
                    Err(format!("Twin '{}' not found", parts[1]))
                }
            }
            
            "set" => {
                // set sensor_1 temperature 25.0
                if parts.len() < 4 {
                    return Err("Usage: set <twin_id> <property> <value>".to_string());
                }
                
                if let Some(twin) = self.twins.get_mut(parts[1]) {
                    let value = match parts[3].parse::<f64>() {
                        Ok(f) => Value::Float(f),
                        Err(_) => match parts[3] {
                            "true" => Value::Boolean(true),
                            "false" => Value::Boolean(false),
                            _ => Value::String(parts[3].to_string()),
                        }
                    };
                    
                    twin.properties.insert(parts[2].to_string(), value);
                    Ok("OK".to_string())
                } else {
                    Err(format!("Twin '{}' not found", parts[1]))
                }
            }
            
            "select" => {
                // select temperature > 22.0
                if parts.len() < 4 {
                    return Err("Usage: select <property> <op> <value>".to_string());
                }
                
                let property = parts[1];
                let op = parts[2];
                let threshold: f64 = parts[3].parse()
                    .map_err(|_| "Invalid number")?;
                
                let mut results = Vec::new();
                for (id, twin) in &self.twins {
                    if let Some(Value::Float(val)) = twin.properties.get(property) {
                        let matches = match op {
                            ">" => *val > threshold,
                            "<" => *val < threshold,
                            "=" | "==" => (*val - threshold).abs() < 0.001,
                            _ => false,
                        };
                        
                        if matches {
                            results.push(format!("{id}: {val}"));
                        }
                    }
                }
                
                Ok(results.join("\n"))
            }
            
            "clone" => {
                // clone sensor_1 as sensor_6
                if parts.len() < 4 || parts[2] != "as" {
                    return Err("Usage: clone <source_id> as <new_id>".to_string());
                }
                
                if let Some(source) = self.twins.get(parts[1]) {
                    let mut new_twin = Twin {
                        id: parts[3].to_string(),
                        class_name: source.class_name.clone(),
                        properties: source.properties.clone(),
                    };
                    
                    self.twins.insert(new_twin.id.clone(), new_twin);
                    Ok(format!("Cloned {} as {}", parts[1], parts[3]))
                } else {
                    Err(format!("Twin '{}' not found", parts[1]))
                }
            }
            
            "help" => {
                Ok("Commands:
  list                          - List all twins
  inspect <id>                  - Show twin details
  get <id> <property>          - Get property value
  set <id> <property> <value>  - Set property value
  select <prop> <op> <value>   - Find twins matching criteria
  clone <id> as <new_id>       - Clone a twin
  quit                         - Exit inspector".to_string())
            }
            
            "quit" | "exit" => {
                Ok("exit".to_string())
            }
            
            _ => Err(format!("Unknown command: {}. Type 'help' for commands.", parts[0]))
        }
    }
}

fn main() {
    println!("=== TwinTalk Inspector ===");
    println!("Minimal Smalltalk-inspired runtime inspection");
    println!("Type 'help' for commands\n");
    
    let mut inspector = TwinInspector::new();
    inspector.create_test_twins();
    
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        match inspector.execute_command(&input) {
            Ok(result) => {
                if result == "exit" {
                    break;
                }
                if !result.is_empty() {
                    println!("{}", result);
                }
            }
            Err(err) => {
                println!("Error: {}", err);
            }
        }
    }
    
    println!("\n=== Performance Notes ===");
    println!("- No parsing overhead for common operations");
    println!("- Direct string matching for commands");
    println!("- Could add Smalltalk syntax sugar without impacting core performance");
    println!("- Real implementation would use the compiled message approach");
}