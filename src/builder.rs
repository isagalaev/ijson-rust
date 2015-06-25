use std::collections::HashMap;

use super::parser::{Event, Filter};


#[derive(Debug)]
#[derive(PartialEq)]
pub enum Value {
    Null,
    Boolean(bool),
    String(String),
    Number(f64),
    Map(self::Map),
    Array(self::Array),
}
pub type Map = HashMap<String, Value>;
pub type Array = Vec<Value>;

fn build(events: &mut Filter) -> Option<Value> {
    match events.next() {
        None => None,
        Some(event) => match event {
            Event::EndMap | Event::EndArray => None,
            Event::StartMap => {
                let mut result = HashMap::new();
                while let Some(event) = events.next() {
                    match event {
                        Event::EndMap => break,
                        Event::Key(k) => {result.insert(k, build(events).unwrap());}
                        _ => unreachable!(),
                    }
                }
                Some(Value::Map(result))
            }
            Event::StartArray => {
                let mut result = vec![];
                while let Some(value) = build(events) {
                    result.push(value);
                }
                Some(Value::Array(result))
            }
            Event::Null => Some(Value::Null),
            Event::Boolean(v) => Some(Value::Boolean(v)),
            Event::String(v) => Some(Value::String(v)),
            Event::Number(v) => Some(Value::Number(v)),
            Event::Key(k) => panic!("Unexpected Key event: {}", k),
        }
    }
}

pub struct Items {
    events: Filter,
}

impl Items {
    pub fn new(events: Filter) -> Items {
        Items {
            events: events,
        }
    }
}

impl Iterator for Items {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        build(&mut self.events)
    }
}
