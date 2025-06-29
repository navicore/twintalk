//! Proof of Concept: Fleet fuel prediction using ADTs and hypothetical twins

use chrono::{Timelike, Utc};
use std::sync::Arc;
use twintalk_core::{
    adt::TeamADT,
    msg,
    runtime::{Runtime, RuntimeConfig},
};

#[tokio::test]
async fn test_fleet_fuel_prediction() {
    // Create runtime
    let runtime = Arc::new(Runtime::new(RuntimeConfig::default()));

    // Create a fleet of delivery trucks
    let mut truck_ids = vec![];
    for i in 0..5 {
        let truck_id = runtime
            .create_twin(&format!("DeliveryTruck_{}", i))
            .await
            .unwrap();
        truck_ids.push(truck_id);

        // Set initial properties
        let truck = runtime.get_twin(truck_id).await.unwrap();
        {
            let mut twin = truck.twin.write().await;
            twin.send(&msg!(fuel_capacity: 100.0)).unwrap();
            twin.send(&msg!(mpg: 8.5)).unwrap();
        }
    }

    // Create Team ADT
    let mut team_adt = TeamADT::new("Team Alpha", truck_ids.clone());

    // Simulate historical data (4 weeks of daily fuel consumption)
    for week in 0..4 {
        for day in 0..7 {
            let date = Utc::now() - chrono::Duration::weeks(week + 1) + chrono::Duration::days(day);
            let hour = date.with_hour(8).unwrap(); // 8 AM

            // Simulate fuel consumption for each truck
            for (i, truck_id) in truck_ids.iter().enumerate() {
                let fuel_consumed = 40.0 + (i as f64 * 5.0) + (day as f64 * 2.0); // Vary by truck and day
                runtime
                    .update_telemetry(*truck_id, vec![("fuel_consumed".to_string(), fuel_consumed)])
                    .await
                    .unwrap();

                // Update ADT metrics
                team_adt.update_metrics(truck_id, "fuel_consumed", fuel_consumed);
            }

            // Create hourly rollup
            team_adt.create_hourly_rollup(hour);
        }
    }

    // Now predict tomorrow's fuel consumption
    let tomorrow = (Utc::now() + chrono::Duration::days(1)).date_naive();
    
    let prediction = team_adt
        .predict_fuel_consumption(&runtime, tomorrow)
        .await
        .unwrap();

    println!("Prediction for {}: {:?}", tomorrow, prediction);

    // Verify prediction is reasonable
    assert!(prediction.total_fuel > 0.0);
    assert!(prediction.confidence > 0.0 && prediction.confidence <= 1.0);
    assert_eq!(prediction.date, tomorrow);
    assert_eq!(prediction.method, "historical_average");

    // Verify hypothetical twins were created but not persisted
    let runtime_clone = runtime.clone();
    let hypothetical_twin_id = runtime_clone
        .create_hypothetical_twin("HypotheticalTruck")
        .await
        .unwrap();

    // Update hypothetical twin
    runtime_clone
        .update_telemetry(
            hypothetical_twin_id,
            vec![("fuel_consumed".to_string(), 999.0)],
        )
        .await
        .unwrap();

    // Verify it's marked as hypothetical
    let h_twin = runtime_clone.get_twin(hypothetical_twin_id).await.unwrap();
    {
        let twin = h_twin.twin.read().await;
        assert!(twin.is_hypothetical());
    }
}

#[tokio::test]
async fn test_adt_partitioning() {
    let runtime = Arc::new(Runtime::new(RuntimeConfig::default()));

    // Create trucks
    let truck1 = runtime.create_twin("Truck1").await.unwrap();
    let truck2 = runtime.create_twin("Truck2").await.unwrap();

    let mut team = TeamADT::new("Test Team", vec![truck1, truck2]);

    // Add some metrics
    team.update_metrics(&truck1, "fuel_consumed", 50.0);
    team.update_metrics(&truck2, "fuel_consumed", 60.0);
    team.update_metrics(&truck1, "miles_driven", 400.0);
    team.update_metrics(&truck2, "miles_driven", 450.0);

    // Create hourly rollup
    let now = Utc::now();
    let rollup = team.create_hourly_rollup(now);

    // Verify rollup
    assert_eq!(rollup.truck_count, 2);
    assert_eq!(rollup.metrics.get("total_fuel_consumed"), Some(&110.0));
    assert_eq!(rollup.metrics.get("total_miles"), Some(&850.0));

    // Verify cache was cleared for next period
    assert_eq!(team.rollup_cache.len(), 0);
}

#[tokio::test]
async fn test_hypothetical_twin_time_manipulation() {
    let runtime = Runtime::new(RuntimeConfig::default());

    // Create hypothetical twin
    let twin_id = runtime.create_hypothetical_twin("TestTwin").await.unwrap();
    let active = runtime.get_twin(twin_id).await.unwrap();

    // Set simulation time to future
    let future_time = Utc::now() + chrono::Duration::days(7);
    {
        let mut twin = active.twin.write().await;
        twin.set_simulation_time(future_time).unwrap();
        assert_eq!(twin.simulation_time(), Some(future_time));
    }

    // Verify it's marked as hypothetical
    {
        let twin = active.twin.read().await;
        assert!(twin.is_hypothetical());
    }
}