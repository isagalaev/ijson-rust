use std::collections::BTreeMap;

use rustc_serialize::json;
use rustc_serialize::json::Json;
use rustc_serialize::Decodable;

use ::parser::Event;


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

pub struct Items<E> where E: Iterator<Item=Event> {
    events: E,
}

impl<E> Iterator for Items<E> where E: Iterator<Item=Event> {
    type Item = Json;

    fn next(&mut self) -> Option<Self::Item> {
        match self.events.next() {
            None => None,
            Some(event) => match event {
                Event::EndMap | Event::EndArray => None,
                Event::StartMap => {
                    let mut result = BTreeMap::new();
                    while let Some(event) = self.events.next() {
                        match event {
                            Event::EndMap => break,
                            Event::Key(k) => {result.insert(k, self.next().unwrap());}
                            _ => unreachable!(),
                        }
                    }
                    Some(Json::Object(result))
                }
                Event::StartArray => {
                    let mut result = vec![];
                    while let Some(value) = self.next() {
                        result.push(value);
                    }
                    Some(Json::Array(result))
                }
                Event::Null => Some(Json::Null),
                Event::Boolean(v) => Some(Json::Boolean(v)),
                Event::String(v) => Some(Json::String(v)),
                Event::Number(v) => Some(Json::F64(v)),
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

pub fn decode<T: Decodable>(json: Json) -> Result<T, json::DecoderError> {
    let mut decoder = json::Decoder::new(json);
    Decodable::decode(&mut decoder)
}
