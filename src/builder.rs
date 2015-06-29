use std::collections::HashMap;

use super::parser::Event;


pub struct Prefix<E> where E: Iterator<Item=Event> {
    reference: Vec<String>,
    path: Vec<String>,
    parser: E,
}

impl<E> Iterator for Prefix<E> where E: Iterator<Item=Event> {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(event) = self.parser.next() {
            match &event {
                &Event::Key(_) | &Event::EndMap | &Event::EndArray => {
                    self.path.pop();
                }
                _ => (),
            }

            let found = self.path.starts_with(&self.reference);

            match &event {
                &Event::Key(ref value) => {
                    self.path.push(value.clone());
                }
                &Event::StartMap => {
                    self.path.push("".to_owned())
                }
                &Event::StartArray => {
                    self.path.push("item".to_owned());
                }
                _ => (),
            }

            if found {
                return Some(event)
            }
        }
        None
    }
}

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

pub struct Items<E> where E: Iterator<Item=Event> {
    events: E,
}

impl<E> Iterator for Items<E> where E: Iterator<Item=Event> {
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

pub trait Builder where Self: Sized + Iterator<Item=Event> {

    fn prefix(self, prefix: &str) -> Prefix<Self>  {
        Prefix {
            reference: prefix.split_terminator(".").map(str::to_string).collect(),
            path: vec![],
            parser: self,
        }
    }

    fn items(self, prefix: &str) -> Items<Prefix<Self>> {
        Items {
            events: self.prefix(prefix),
        }
    }
}

impl<T> Builder for T where T: Sized + Iterator<Item=Event> {}
