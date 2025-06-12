//! TwinTalk HTTP API Server
//! 
//! REST and WebSocket API for interacting with the twin runtime:
//! - Create, clone, and manage twin instances
//! - Stream telemetry data to twins
//! - Query twin state and subscribe to changes
//! - Execute Smalltalk code on twins

pub mod routes;
pub mod handlers;
pub mod websocket;
pub mod error;

use axum::Router;
use twintalk_core::Runtime;

pub async fn create_app(runtime: Runtime) -> Router {
    routes::create_router(runtime)
}