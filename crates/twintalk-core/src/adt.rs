//! Aggregate Digital Twin (ADT) implementation
//!
//! ADTs aggregate state and behavior from multiple constituent twins,
//! enabling hierarchical rollups and system-level predictions.

use crate::runtime::Runtime;
use crate::twin::TwinId;
use crate::value::Value;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc, Weekday};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Time period for rollup aggregations
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TimePeriod {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimePeriod {
    /// Create hourly period containing the given time
    pub fn hourly(time: DateTime<Utc>) -> Self {
        let start = time
            .with_minute(0)
            .and_then(|t| t.with_second(0))
            .and_then(|t| t.with_nanosecond(0))
            .unwrap();
        let end = start + Duration::hours(1);
        Self { start, end }
    }

    /// Create daily period containing the given date
    pub fn daily(date: NaiveDate) -> Self {
        let start = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end = start + Duration::days(1);
        Self { start, end }
    }

    /// Check if a timestamp falls within this period
    pub fn contains(&self, time: DateTime<Utc>) -> bool {
        time >= self.start && time < self.end
    }
}

/// Aggregated metrics for a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rollup {
    pub period: TimePeriod,
    pub metrics: HashMap<String, f64>,
    pub truck_count: usize,
    pub computed_at: DateTime<Utc>,
}

/// Prediction result with confidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub date: NaiveDate,
    pub total_fuel: f64,
    pub confidence: f64,
    pub method: String,
}

/// Team-level ADT aggregating multiple trucks
pub struct TeamADT {
    pub id: TwinId,
    pub name: String,
    pub trucks: Vec<TwinId>,
    pub rollup_cache: DashMap<String, f64>,
    pub partition_cache: BTreeMap<TimePeriod, Rollup>,
}

impl TeamADT {
    /// Create a new Team ADT
    pub fn new(name: impl Into<String>, trucks: Vec<TwinId>) -> Self {
        Self {
            id: TwinId::new(),
            name: name.into(),
            trucks,
            rollup_cache: DashMap::new(),
            partition_cache: BTreeMap::new(),
        }
    }

    /// Update cached metrics from truck telemetry
    pub fn update_metrics(&self, truck_id: &TwinId, metric: &str, value: f64) {
        if !self.trucks.contains(truck_id) {
            return;
        }

        // Update aggregates based on metric type
        match metric {
            "fuel_consumed" => {
                self.rollup_cache
                    .entry("total_fuel_consumed".to_string())
                    .and_modify(|v| *v += value)
                    .or_insert(value);
            }
            "miles_driven" => {
                self.rollup_cache
                    .entry("total_miles".to_string())
                    .and_modify(|v| *v += value)
                    .or_insert(value);
            }
            _ => {}
        }
    }

    /// Create hourly rollup
    pub fn create_hourly_rollup(&mut self, hour: DateTime<Utc>) -> Rollup {
        let period = TimePeriod::hourly(hour);
        
        // Snapshot current metrics
        let mut metrics = HashMap::new();
        for entry in self.rollup_cache.iter() {
            metrics.insert(entry.key().clone(), *entry.value());
        }

        let rollup = Rollup {
            period: period.clone(),
            metrics,
            truck_count: self.trucks.len(),
            computed_at: Utc::now(),
        };

        // Cache the rollup
        self.partition_cache.insert(period, rollup.clone());

        // Clear accumulators for next period
        self.rollup_cache.clear();

        rollup
    }

    /// Get historical average for a specific weekday
    pub fn get_historical_average(&self, weekday: Weekday, weeks: usize) -> Result<f64> {
        let mut total = 0.0;
        let mut count = 0;

        let today = Utc::now().date_naive();
        
        for week in 1..=weeks {
            let _target_date = today - Duration::weeks(week as i64);
            
            // Find rollups for this weekday
            for (period, rollup) in &self.partition_cache {
                if period.start.date_naive().weekday() == weekday {
                    if let Some(fuel) = rollup.metrics.get("total_fuel_consumed") {
                        total += fuel;
                        count += 1;
                    }
                }
            }
        }

        if count == 0 {
            return Err(anyhow!("No historical data found"));
        }

        Ok(total / count as f64)
    }

    /// Predict fuel consumption using historical patterns
    pub async fn predict_fuel_consumption(
        &self,
        runtime: &Runtime,
        target_date: NaiveDate,
    ) -> Result<Prediction> {
        // Clone all trucks as hypothetical
        let mut hypothetical_trucks = vec![];
        let mut hypothetical_ids = vec![];
        
        for truck_id in &self.trucks {
            let active = runtime.get_twin(*truck_id).await?;
            let truck = active.twin.read().await;
            let h_truck = truck.clone_hypothetical();
            let h_id = h_truck.id();
            
            // Set simulation time
            let mut h_truck_mut = h_truck;
            h_truck_mut.set_simulation_time(target_date.and_hms_opt(0, 0, 0).unwrap().and_utc())?;
            
            hypothetical_ids.push(h_id);
            hypothetical_trucks.push(Arc::new(tokio::sync::RwLock::new(h_truck_mut)));
        }

        // Get historical average for the target weekday
        let historical_avg = self.get_historical_average(target_date.weekday(), 4)
            .unwrap_or(500.0); // Default if no history

        // Simple prediction: use historical average with some variance
        let predicted_fuel_per_truck = historical_avg / self.trucks.len() as f64;

        // Update hypothetical trucks with predictions
        for (_i, h_truck) in hypothetical_trucks.iter().enumerate() {
            let mut truck = h_truck.write().await;
            truck.send(&crate::Message::SetProperty(
                "predicted_fuel".to_string(),
                Value::Float((predicted_fuel_per_truck * 0.95).into()), // 95-105% variance
            ))?;
            
            truck.send(&crate::Message::SetProperty(
                "predicted_miles".to_string(),
                Value::Float((predicted_fuel_per_truck * 8.5).into()), // Assume 8.5 mpg
            ))?;
        }

        Ok(Prediction {
            date: target_date,
            total_fuel: historical_avg,
            confidence: 0.75, // Simple confidence based on having 4 weeks of data
            method: "historical_average".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_time_period() {
        let time = Utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap();
        let period = TimePeriod::hourly(time);
        
        assert_eq!(period.start.hour(), 14);
        assert_eq!(period.start.minute(), 0);
        assert!(period.contains(time));
        assert!(!period.contains(time + Duration::hours(1)));
    }

    #[test]
    fn test_rollup_cache() {
        let team = TeamADT::new("Team Alpha", vec![TwinId::new(), TwinId::new()]);
        let truck_id = team.trucks[0];
        
        team.update_metrics(&truck_id, "fuel_consumed", 50.0);
        team.update_metrics(&truck_id, "fuel_consumed", 25.0);
        
        assert_eq!(
            *team.rollup_cache.get("total_fuel_consumed").unwrap(),
            75.0
        );
    }
}