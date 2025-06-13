# Minimal Smalltalk for Digital Twins

## Design Philosophy

"Just enough Smalltalk" - We want the elegance of message passing and runtime inspection without the overhead of a full language implementation.

## Essential Smalltalk Features for Twins

### 1. Core Message Passing
```smalltalk
"Must have"
sensor temperature           "getter"
sensor temperature: 25.0     "setter"
sensor updateFrom: telemetry "method call"
```

### 2. Blocks for Behavior
```smalltalk
"Simple blocks for conditions and actions"
sensor whenAlert: [console log: 'Temperature exceeded!']
sensor checkIf: [:temp | temp > threshold]
```

### 3. Prototype Operations
```smalltalk
"Cloning and customization"
newSensor := sensor clone
newSensor threshold: 28.0
```

### 4. Runtime Inspection
```smalltalk
"Essential for debugging/monitoring"
sensor class               "TemperatureSensor"
sensor allProperties       "Dictionary of state"
sensor respondsTo: #update "true/false"
```

## What We DON'T Need

- Complex class hierarchies
- Metaclasses
- Method compilation at runtime
- Full collection protocols
- Exception handling (use Rust's Result)
- Workspace/Transcript UI

## Implementation Strategy

### Option 1: Direct Dispatch (Fastest)
- Parse Smalltalk to Rust enum at compile time
- No runtime parsing for common operations
- ~0 overhead for pre-compiled messages

```rust
enum Message {
    GetProperty(String),
    SetProperty(String, Value),
    UpdateTelemetry(HashMap<String, f64>),
    // ... other common messages
}
```

### Option 2: Bytecode VM (Flexible)
- Compile Smalltalk to simple bytecode
- Cache compiled methods
- ~100ns overhead (still fast)

```rust
enum Bytecode {
    LoadSelf,
    LoadLiteral(Value),
    Send(String, u8), // selector, arg count
    Return,
}
```

### Option 3: Hybrid Approach (Recommended)
- Fast path: Direct dispatch for common operations
- Slow path: Interpreted for runtime-defined behavior
- Best of both worlds

## Parsing Strategy

### 1. Two-Phase Parsing
```rust
// Phase 1: At twin definition time (once)
let twin_class = parse_smalltalk_class(source)?;
let compiled = compile_to_dispatch_table(twin_class)?;

// Phase 2: At runtime (never parse)
twin.send_message(Message::GetProperty("temperature"))
```

### 2. Message Caching
```rust
// Cache parsed messages
lazy_static! {
    static ref MESSAGE_CACHE: DashMap<String, Message> = DashMap::new();
}

fn parse_and_cache(msg_text: &str) -> Message {
    MESSAGE_CACHE.entry(msg_text.to_string())
        .or_insert_with(|| parse_message(msg_text))
        .clone()
}
```

### 3. Compile-Time Optimization
```rust
// Macro for zero-cost message sends
smalltalk!(sensor.temperature: 25.0)
// Expands to:
sensor.send_message(Message::SetProperty("temperature".into(), Value::Float(25.0)))
```

## Runtime Inspection Design

### 1. REPL for Live Twins
```
> inspect sensor_42
Twin: sensor_42
Class: TemperatureSensor
State:
  temperature: 24.5
  threshold: 30.0
  alert: false

> sensor_42 temperature
24.5

> sensor_42 temperature: 26.0
ok

> sensor_42 clone as: sensor_43
Twin sensor_43 created
```

### 2. Query Language
```smalltalk
"Find all twins matching criteria"
twins select: [:t | t temperature > 25.0]
twins detect: [:t | t isAlerting]
```

## Performance Targets

| Operation | Target | Strategy |
|-----------|--------|----------|
| Property get/set | <100ns | Direct dispatch |
| Method call | <200ns | Cached bytecode |
| Parse & compile | <1ms | One-time cost |
| Clone twin | <1μs | Rust prototype |
| Inspect state | <100μs | Reflection API |

## Example Minimal Implementation

```rust
// Core twin trait with Smalltalk semantics
trait SmalltalkTwin {
    fn send(&mut self, selector: &str, args: &[Value]) -> Result<Value, String>;
    fn get_slot(&self, name: &str) -> Option<&Value>;
    fn set_slot(&mut self, name: &str, value: Value);
    fn clone_twin(&self) -> Box<dyn SmalltalkTwin>;
}

// Fast message dispatch
impl Twin {
    fn send(&mut self, selector: &str, args: &[Value]) -> Result<Value, String> {
        match selector {
            // Fast path - no parsing
            "temperature" => Ok(self.temperature.into()),
            "temperature:" => {
                self.temperature = args[0].as_float()?;
                Ok(Value::Nil)
            }
            // Slow path - dynamic lookup
            _ => self.lookup_method(selector)?.call(self, args)
        }
    }
}
```

## Next Steps

1. Implement minimal parser for subset
2. Benchmark direct dispatch vs bytecode
3. Build inspection REPL
4. Create macro system for compile-time optimization