//! TwinTalk Supervisor
//! 
//! Erlang-style supervision trees for twin lifecycle management:
//! - Automatic restart strategies (one-for-one, one-for-all, rest-for-one)
//! - Health monitoring and failure detection
//! - Resource limits and backpressure
//! - Twin spawn/despawn orchestration

pub mod supervisor;
pub mod strategy;
pub mod child_spec;
pub mod restart;

pub use supervisor::{Supervisor, SupervisorConfig};
pub use strategy::RestartStrategy;
pub use child_spec::ChildSpec;