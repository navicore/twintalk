# TwinTalk - Digital Twin Runtime

An experimental digital twin runtime combining Rust performance with Smalltalk expressiveness for prototype-based twin programming.

## Concept
- **Digital Twins**: Software representations that mirror real-world entities through telemetry
- **Prototype-Based**: Clone and customize twin instances dynamically
- **Smalltalk DSL**: Expressive language for twin behavior definition
- **Rust Runtime**: High-performance, safe execution environment

## Current Status
🔬 **Feasibility Investigation Phase**

See [FEASIBILITY_PLAN.md](FEASIBILITY_PLAN.md) for detailed investigation plan.

## Architecture

The system is organized as a Cargo workspace with three main crates:

- **twintalk-core**: Core runtime engine with twin execution, state management, and Smalltalk DSL interpreter
- **twintalk-api**: HTTP/WebSocket API server for external integration
- **twintalk-supervisor**: Erlang-style supervisor for twin lifecycle, fault tolerance, and restart strategies

## Repository Structure
```
twintalk/
├── crates/
│   ├── twintalk-core/      # Core runtime engine
│   ├── twintalk-api/       # HTTP API server
│   └── twintalk-supervisor/ # Supervisor system
├── examples/               # Twin definition examples
├── benches/               # Performance benchmarks
└── docs/                  # Architecture documentation
```

## Getting Started

```bash
# Build all crates
cargo build

# Run tests
cargo test

# Run benchmarks
cargo bench
```