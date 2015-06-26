use std::collections::HashMap;

use super::parser::Event;


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

pub struct Items<I> where I: Iterator<Item=Event> {
    events: I,
}

impl<I> Items<I> where I: Iterator<Item=Event> {
    pub fn new(events: I) -> Items<I> {
        Items {
            events: events,
        }
    }
}

impl<I> Iterator for Items<I> where I: Iterator<Item=Event> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self.events.next() {
            None => None,
            Some(event) => match event {
                Event::EndMap | Event::EndArray => None,
                Event::StartMap => {
                    let mut result = HashMap::new();
                    while let Some(event) = self.events.next() {
                        match event {
                            Event::EndMap => break,
                            Event::Key(k) => {result.insert(k, self.next().unwrap());}
                            _ => unreachable!(),
                        }
                    }
                    Some(Value::Map(result))
                }
                Event::StartArray => {
                    let mut result = vec![];
                    while let Some(value) = self.next() {
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
}
