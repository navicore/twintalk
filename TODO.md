# TwinTalk TODO

## Completed
- [x] Core runtime implementation with:
  - Message dispatch system
  - Value types
  - Twin prototype-based cloning
  - Event sourcing with lazy loading
  - Comprehensive test suite (43 tests)

## Pending Implementation

### HTTP API Server (`twintalk-api`)
Currently a placeholder to allow compilation. Needs:
- REST endpoints for twin management
- WebSocket support for real-time telemetry
- Authentication and authorization
- OpenAPI documentation

### Supervisor (`twintalk-supervisor`)
Currently a placeholder to allow compilation. Needs:
- Supervision tree implementation
- Restart strategies (one-for-one, one-for-all, rest-for-one)
- Health monitoring
- Resource limits and backpressure

### REPL
Low priority task for interactive twin inspection:
- Parse and execute Smalltalk messages
- Browse twin state
- Debug message dispatch

## Build Notes
- Fixed `.gitignore` to properly handle Cargo workspace structure
- Use `cargo test --workspace` to run all tests
- Use `cargo test --package twintalk-core` to run only core tests