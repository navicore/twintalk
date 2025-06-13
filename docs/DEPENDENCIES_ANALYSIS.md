# TwinTalk Dependencies Analysis

## Core Runtime (twintalk-core)

### Essential Dependencies

1. **Message Passing & State** - Pure Rust
   - No dependencies needed! Use enums and trait dispatch

2. **Event Storage**
   ```toml
   sled = "0.34"        # Embedded event log DB (or alternatives below)
   bincode = "1.3"      # Fast binary serialization for events
   ```

3. **Concurrent State Management**
   ```toml
   dashmap = "5.5"      # Concurrent HashMap for twin registry
   arc-swap = "1.6"     # Atomic pointer swapping for hot reloading
   ```

4. **Async Runtime**
   ```toml
   tokio = { version = "1", features = ["rt-multi-thread", "sync"] }
   ```

### Optional: Minimal Smalltalk Parser
```toml
nom = "7"            # If we want parser combinators
# OR just use Rust pattern matching!
```

### What We DON'T Need
- ❌ Full Smalltalk VM (PharoVM, etc.)
- ❌ Heavy scripting engines (wasmer, wasmtime)
- ❌ Complex parser generators (pest, lalrpop)
- ❌ Actor frameworks (actix, bastion)

## Alternative Storage Options

Instead of sled, could use:
```toml
# Option 1: Even simpler
redb = "1.0"         # Pure Rust embedded DB

# Option 2: If you need SQL
rusqlite = "0.29"    # SQLite for event log

# Option 3: Just files
# Use tokio::fs with bincode - no DB needed!
```

## Minimal Complete Cargo.toml

```toml
[dependencies]
# Async runtime (required)
tokio = { version = "1", features = ["rt-multi-thread", "sync", "fs"] }
async-trait = "0.1"

# Serialization (required)
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"      # For event log

# Concurrent collections (required)
dashmap = "5.5"

# Event storage (pick one)
sled = "0.34"        # Recommended: embedded, fast
# OR redb = "1.0"    # Alternative: pure Rust
# OR just use files!

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# That's it! Everything else is optional
```

## Implementation Strategy

### 1. Pure Rust Message Dispatch
```rust
// No dependencies needed!
enum Message {
    Get(&'static str),
    Set(&'static str, Value),
    Update(Vec<(String, f64)>),
}

impl Twin {
    fn send(&mut self, msg: Message) -> Result<Value> {
        match msg {
            Message::Get(prop) => Ok(self.props.get(prop).cloned()),
            Message::Set(prop, val) => {
                self.props.insert(prop.to_string(), val);
                Ok(Value::Nil)
            }
            // ...
        }
    }
}
```

### 2. Simple Bytecode (Optional)
```rust
// Also pure Rust!
enum Op {
    LoadSlot(u8),
    StoreSlot(u8),
    LoadConst(u8),
    Send(u8, u8),  // method_id, arg_count
    Return,
}

// No external VM needed!
```

### 3. Event Persistence
```rust
// With sled (or redb):
let event = bincode::serialize(&twin_event)?;
db.insert(&twin_id.to_bytes(), event)?;

// Or just files:
let path = format!("events/{}.bin", twin_id);
tokio::fs::write(&path, &event).await?;
```

## Parsing Strategy

### Option A: Pure Rust Pattern Matching
```rust
// No parser library needed!
fn parse_message(input: &str) -> Result<Message> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    match parts.as_slice() {
        [obj, prop] => Ok(Message::Get(prop)),
        [obj, prop, ":", value] => {
            let val = parse_value(value)?;
            Ok(Message::Set(prop, val))
        }
        _ => Err("Invalid message".into())
    }
}
```

### Option B: Nom for Complex Syntax
```rust
use nom::{
    branch::alt,
    bytes::complete::tag,
    // ... minimal combinators
};

// Only if we need blocks, cascades, etc.
```

## Summary

**Required dependencies:** ~5-6 crates
**Optional dependencies:** 1-2 for parsing
**No heavy frameworks needed!**

The beauty is that Rust's built-in features (enums, pattern matching, traits) give us most of what we need for a message-passing system. We're not building a general-purpose language - just a domain-specific twin programming system.