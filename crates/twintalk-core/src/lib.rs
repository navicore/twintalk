//! TwinTalk Core Runtime
//! 
//! This crate provides the core digital twin execution engine with:
//! - Twin instance management and prototype-based cloning
//! - Smalltalk DSL interpreter for twin behaviors
//! - Telemetry ingestion and state updates
//! - Event-driven message passing between twins

pub mod twin;
pub mod interpreter;
pub mod telemetry;
pub mod prototype;
pub mod runtime;

pub use runtime::Runtime;
pub use twin::{Twin, TwinId};