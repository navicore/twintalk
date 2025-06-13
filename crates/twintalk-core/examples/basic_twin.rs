//! Basic example of creating and using digital twins

use std::sync::Arc;
use twintalk_core::{msg, Runtime, RuntimeConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize runtime
    let runtime = Arc::new(Runtime::new(RuntimeConfig::default()));

    println!("=== TwinTalk Basic Example ===\n");

    // Create a temperature sensor twin
    let sensor_id = runtime.create_twin("TemperatureSensor").await?;
    println!("Created twin: {}", sensor_id);

    // Send telemetry
    println!("\nSending telemetry...");
    runtime
        .update_telemetry(
            sensor_id,
            vec![
                ("temperature".to_string(), 22.5),
                ("humidity".to_string(), 45.0),
                ("threshold".to_string(), 30.0),
            ],
        )
        .await?;

    // Get the twin and query its state
    let active = runtime.get_twin(sensor_id).await?;
    let mut twin = active.twin.write().await;

    println!("\nQuerying twin state:");
    let temp = twin.send(&msg!(temperature))?;
    println!("  Temperature: {}", temp);

    let humidity = twin.send(&msg!(humidity))?;
    println!("  Humidity: {}", humidity);

    // Test message passing
    println!("\nTesting message passing:");
    let class_name = twin.send(&msg!(class))?;
    println!("  Class: {}", class_name);

    let all_props = twin.send(&msg!(allProperties))?;
    println!("  All properties: {}", all_props);

    // Check alert
    let alert = twin.send(&twintalk_core::Message::Send {
        selector: "checkAlert".to_string(),
        args: vec![],
    })?;
    println!("  Alert status: {}", alert);

    // Clone the twin
    println!("\nCloning twin...");
    let cloned = twin.clone_twin();
    println!("  Original ID: {}", twin.id());
    println!("  Cloned ID: {}", cloned.id());

    // Update telemetry to trigger alert
    drop(twin); // Release the lock
    println!("\nUpdating temperature above threshold...");
    runtime
        .update_telemetry(sensor_id, vec![("temperature".to_string(), 35.0)])
        .await?;

    // Check alert again
    let active = runtime.get_twin(sensor_id).await?;
    let mut twin = active.twin.write().await;
    let alert = twin.send(&twintalk_core::Message::Send {
        selector: "checkAlert".to_string(),
        args: vec![],
    })?;
    println!("  Alert status after update: {}", alert);

    // Get runtime stats
    drop(twin);
    let stats = runtime.stats().await;
    println!("\nRuntime statistics:");
    println!("  Active twins: {}", stats.active_twins);
    println!("  Total events: {}", stats.total_events);

    Ok(())
}
