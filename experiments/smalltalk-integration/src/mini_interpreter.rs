//! Minimal Smalltalk interpreter experiment
//! 
//! Tests feasibility of building a tiny Smalltalk subset for twins

use nom::{
    IResult,
    branch::alt,
    bytes::complete::take_while1,
    character::complete::{char, digit1},
    combinator::{map, recognize},
    sequence::{delimited, preceded, tuple},
};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Symbol(String),
    Object(Rc<Object>),
    Block(Vec<String>, Box<Expr>),
    Nil,
}

#[derive(Debug)]
struct Object {
    class: String,
    slots: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
enum Expr {
    Literal(Value),
    Variable(String),
    MessageSend {
        receiver: Box<Expr>,
        selector: String,
        args: Vec<Expr>,
    },
    Block {
        params: Vec<String>,
        body: Box<Expr>,
    },
}

// Basic parser for messages like: "sensor temperature: 25.0"
fn parse_identifier(input: &str) -> IResult<&str, String> {
    map(
        take_while1(|c: char| c.is_alphanumeric() || c == '_'),
        String::from
    )(input)
}

fn parse_number(input: &str) -> IResult<&str, Value> {
    alt((
        map(
            recognize(tuple((digit1, char('.'), digit1))),
            |s: &str| Value::Float(s.parse().unwrap())
        ),
        map(digit1, |s: &str| Value::Integer(s.parse().unwrap()))
    ))(input)
}

fn parse_string(input: &str) -> IResult<&str, Value> {
    map(
        delimited(char('\''), take_while1(|c| c != '\''), char('\'')),
        |s: &str| Value::String(s.to_string())
    )(input)
}

fn parse_symbol(input: &str) -> IResult<&str, Value> {
    map(
        preceded(char('#'), parse_identifier),
        Value::Symbol
    )(input)
}

// Simple evaluator
struct Interpreter {
    globals: HashMap<String, Value>,
}

impl Interpreter {
    fn new() -> Self {
        let mut globals = HashMap::new();
        
        // Bootstrap some basic objects
        globals.insert("nil".to_string(), Value::Nil);
        
        Self { globals }
    }
    
    fn eval(&mut self, expr: &Expr, locals: &HashMap<String, Value>) -> Value {
        match expr {
            Expr::Literal(v) => v.clone(),
            Expr::Variable(name) => {
                locals.get(name)
                    .or_else(|| self.globals.get(name))
                    .cloned()
                    .unwrap_or(Value::Nil)
            }
            Expr::MessageSend { receiver, selector, args } => {
                let recv_val = self.eval(receiver, locals);
                self.send_message(recv_val, selector, args, locals)
            }
            Expr::Block { params, body } => {
                Value::Block(params.clone(), body.clone())
            }
        }
    }
    
    fn send_message(&mut self, receiver: Value, selector: &str, args: &[Expr], locals: &HashMap<String, Value>) -> Value {
        // Handle primitive messages
        match (&receiver, selector) {
            (Value::Integer(n), "+") => {
                if let Value::Integer(m) = self.eval(&args[0], locals) {
                    Value::Integer(n + m)
                } else {
                    Value::Nil
                }
            }
            (Value::Float(n), "+") => {
                if let Value::Float(m) = self.eval(&args[0], locals) {
                    Value::Float(n + m)
                } else {
                    Value::Nil
                }
            }
            (Value::Integer(n), ">") => {
                if let Value::Integer(m) = self.eval(&args[0], locals) {
                    if n > &m { Value::Symbol("true".to_string()) } else { Value::Symbol("false".to_string()) }
                } else {
                    Value::Nil
                }
            }
            _ => Value::Nil
        }
    }
}

fn main() {
    println!("=== Mini Smalltalk Interpreter Experiment ===\n");
    
    let mut interpreter = Interpreter::new();
    
    // Test basic arithmetic
    let expr = Expr::MessageSend {
        receiver: Box::new(Expr::Literal(Value::Integer(10))),
        selector: "+".to_string(),
        args: vec![Expr::Literal(Value::Integer(5))],
    };
    
    let result = interpreter.eval(&expr, &HashMap::new());
    println!("10 + 5 = {:?}", result);
    
    // Test comparison
    let expr = Expr::MessageSend {
        receiver: Box::new(Expr::Literal(Value::Integer(10))),
        selector: ">".to_string(),
        args: vec![Expr::Literal(Value::Integer(5))],
    };
    
    let result = interpreter.eval(&expr, &HashMap::new());
    println!("10 > 5 = {:?}", result);
    
    // Measure message send overhead
    use std::time::Instant;
    let start = Instant::now();
    for _ in 0..1_000_000 {
        let expr = Expr::MessageSend {
            receiver: Box::new(Expr::Literal(Value::Integer(10))),
            selector: "+".to_string(),
            args: vec![Expr::Literal(Value::Integer(5))],
        };
        interpreter.eval(&expr, &HashMap::new());
    }
    let elapsed = start.elapsed();
    println!("\n1M message sends took: {:?}", elapsed);
    println!("Average: {:?} per message", elapsed / 1_000_000);
}