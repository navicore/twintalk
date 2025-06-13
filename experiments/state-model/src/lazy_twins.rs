//! Lazy twin instantiation patterns
//! 
//! Demonstrates different strategies for lazy loading and memory management

use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
struct TwinId(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TwinData {
    id: TwinId,
    class: String,
    state: serde_json::Map<String, serde_json::Value>,
    last_update: DateTime<Utc>,
    #[serde(skip, default = "Instant::now")]
    last_accessed: Instant,
}

// Different loading strategies
#[derive(Debug, Clone)]
enum LoadStrategy {
    // Load immediately when referenced
    Eager,
    // Load only when state is accessed
    Lazy,
    // Keep hot twins in memory, cold ones on disk
    Adaptive { hot_threshold: Duration },
}

// Loader abstraction
#[async_trait::async_trait]
trait TwinLoader: Send + Sync {
    async fn load(&self, id: &TwinId) -> Result<TwinData, String>;
    async fn save(&self, data: &TwinData) -> Result<(), String>;
}

// Simple file-based loader for demo
struct FileLoader {
    base_path: String,
}

#[async_trait::async_trait]
impl TwinLoader for FileLoader {
    async fn load(&self, id: &TwinId) -> Result<TwinData, String> {
        // Simulate loading from disk
        tokio::time::sleep(Duration::from_micros(100)).await;
        
        Ok(TwinData {
            id: id.clone(),
            class: "TemperatureSensor".to_string(),
            state: serde_json::json!({
                "temperature": 20.0,
                "threshold": 30.0
            }).as_object().unwrap().clone(),
            last_update: Utc::now(),
            last_accessed: Instant::now(),
        })
    }

    async fn save(&self, _data: &TwinData) -> Result<(), String> {
        // Simulate saving to disk
        tokio::time::sleep(Duration::from_micros(50)).await;
        Ok(())
    }
}

// Lazy reference to a twin
struct LazyTwin {
    id: TwinId,
    data: ArcSwap<Option<TwinData>>,
    loader: Arc<dyn TwinLoader>,
    strategy: LoadStrategy,
}

impl LazyTwin {
    fn new(id: TwinId, loader: Arc<dyn TwinLoader>, strategy: LoadStrategy) -> Self {
        Self {
            id,
            data: ArcSwap::new(Arc::new(None)),
            loader,
            strategy,
        }
    }

    async fn ensure_loaded(&self) -> Result<(), String> {
        let current = self.data.load();
        if current.is_none() {
            let data = self.loader.load(&self.id).await?;
            self.data.store(Arc::new(Some(data)));
        }
        Ok(())
    }

    async fn get_state(&self, key: &str) -> Result<Option<serde_json::Value>, String> {
        self.ensure_loaded().await?;
        
        let data = self.data.load();
        Ok(data.as_ref()
            .as_ref()
            .and_then(|d| d.state.get(key).cloned()))
    }

    async fn update_state(&self, key: String, value: serde_json::Value) -> Result<(), String> {
        self.ensure_loaded().await?;
        
        // Clone current state
        let current = self.data.load();
        if let Some(data) = current.as_ref() {
            let mut new_data = data.clone();
            new_data.state.insert(key, value);
            new_data.last_update = Utc::now();
            new_data.last_accessed = Instant::now();
            
            // Update in memory
            self.data.store(Arc::new(Some(new_data.clone())));
            
            // Persist based on strategy
            match &self.strategy {
                LoadStrategy::Eager => {
                    self.loader.save(&new_data).await?;
                }
                LoadStrategy::Lazy => {
                    // Don't save immediately
                }
                LoadStrategy::Adaptive { .. } => {
                    // Save important updates immediately
                    self.loader.save(&new_data).await?;
                }
            }
        }
        
        Ok(())
    }

    fn evict(&self) {
        self.data.store(Arc::new(None));
    }
}

// Runtime managing all lazy twins
struct LazyTwinRuntime {
    twins: DashMap<TwinId, Arc<LazyTwin>>,
    loader: Arc<dyn TwinLoader>,
    strategy: LoadStrategy,
}

impl LazyTwinRuntime {
    fn new(loader: Arc<dyn TwinLoader>, strategy: LoadStrategy) -> Self {
        Self {
            twins: DashMap::new(),
            loader,
            strategy,
        }
    }

    async fn get_or_create(&self, id: TwinId) -> Arc<LazyTwin> {
        self.twins
            .entry(id.clone())
            .or_insert_with(|| {
                Arc::new(LazyTwin::new(
                    id,
                    self.loader.clone(),
                    self.strategy.clone(),
                ))
            })
            .clone()
    }

    async fn process_telemetry(&self, id: TwinId, data: serde_json::Value) -> Result<(), String> {
        let twin = self.get_or_create(id).await;
        
        if let Some(obj) = data.as_object() {
            for (k, v) in obj {
                twin.update_state(k.clone(), v.clone()).await?;
            }
        }
        
        Ok(())
    }

    async fn evict_cold_twins(&self, threshold: Duration) {
        let now = Instant::now();
        let mut to_evict = Vec::new();

        for entry in self.twins.iter() {
            if let Some(data) = entry.value().data.load().as_ref() {
                if now.duration_since(data.last_accessed) > threshold {
                    to_evict.push(entry.key().clone());
                }
            }
        }

        for id in to_evict {
            if let Some((_, twin)) = self.twins.remove(&id) {
                twin.evict();
            }
        }
    }
}

#[tokio::main]
async fn main() {
    println!("=== Lazy Twin Loading Patterns ===\n");

    let loader = Arc::new(FileLoader {
        base_path: "/tmp/twins".to_string(),
    });

    // Test different strategies
    let strategies = vec![
        ("Eager", LoadStrategy::Eager),
        ("Lazy", LoadStrategy::Lazy),
        ("Adaptive", LoadStrategy::Adaptive { 
            hot_threshold: Duration::from_secs(30) 
        }),
    ];

    for (name, strategy) in strategies {
        println!("\n--- {} Strategy ---", name);
        
        let runtime = LazyTwinRuntime::new(loader.clone(), strategy);
        
        // Create twin IDs
        let twin_ids: Vec<_> = (0..1000)
            .map(|i| TwinId(format!("sensor_{}", i)))
            .collect();

        // Process telemetry for subset
        let start = Instant::now();
        for i in 0..100 {
            let data = serde_json::json!({
                "temperature": 25.0 + i as f64 * 0.1,
            });
            runtime.process_telemetry(twin_ids[i].clone(), data).await.unwrap();
        }
        let elapsed = start.elapsed();
        
        println!("Processed 100 telemetry updates in {:?}", elapsed);
        println!("Average per update: {:?}", elapsed / 100);
        println!("Twins in memory: {}", runtime.twins.len());

        // Access pattern test
        let start = Instant::now();
        for i in 900..910 {
            let twin = runtime.get_or_create(twin_ids[i].clone()).await;
            let _temp = twin.get_state("temperature").await.unwrap();
        }
        let elapsed = start.elapsed();
        
        println!("Cold load 10 twins: {:?}", elapsed);
        println!("Average cold load: {:?}", elapsed / 10);

        // Memory pressure simulation
        runtime.evict_cold_twins(Duration::from_secs(0)).await;
        println!("After eviction: {} twins in memory", runtime.twins.len());
    }

    println!("\n=== Memory Efficiency Comparison ===");
    println!("Lazy loading advantages:");
    println!("- Only load twins that receive telemetry");
    println!("- Automatic eviction of cold twins");
    println!("- Configurable persistence strategies");
    println!("- No wasted memory on dormant twins");
    
    println!("\nVs Actor model:");
    println!("- Actors stay in memory even when idle");
    println!("- Need explicit passivation logic");
    println!("- Mailbox memory overhead per actor");
    println!("- Supervision adds memory pressure");
}