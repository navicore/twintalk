//! `TwinTalk` Core Runtime
//!
//! This crate provides the core digital twin execution engine with:
//! - Twin instance management and prototype-based cloning
//! - `Smalltalk`-inspired message passing
//! - Telemetry ingestion and state updates
//! - Event sourcing for persistence

#![allow(clippy::multiple_crate_versions)]

pub mod event;
pub mod message;
pub mod runtime;
pub mod storage;
pub mod twin;
pub mod value;

pub use message::Message;
pub use runtime::{Runtime, RuntimeConfig};
pub use twin::{Twin, TwinId};
pub use value::Value;

// Re-export the message macro
#[doc(hidden)]
pub use crate as twintalk_core;
