use std::collections::BTreeMap;
use std::result;

use rustc_serialize::json;
use rustc_serialize::json::Json;
use rustc_serialize::Decodable;

use ::parser::Event;
use ::errors::Result;


pub trait EventIterator: Iterator<Item=Result<Event>> {}
impl<T: Iterator<Item=Result<Event>>> EventIterator for T {}

pub struct Prefix<E: EventIterator> {
    reference: Vec<String>,
    path: Vec<String>,
    parser: E,
}

impl<E: EventIterator> Iterator for Prefix<E> {
    type Item = Result<Event>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(r) = self.parser.next() {
            let event = itry!(r);
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
                return Some(Ok(event))
            }
        }
        None
    }
}

pub struct Items<E> where E: EventIterator {
    events: E,
}

impl<E> Iterator for Items<E> where E: EventIterator {
    type Item = Result<Json>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.events.next() {
            None => None,
            Some(result) => match itry!(result) {
                Event::EndMap | Event::EndArray => None,
                Event::StartMap => {
                    let mut object = BTreeMap::new();
                    while let Some(result) = self.events.next() {
                        match itry!(result) {
                            Event::EndMap => break,
                            Event::Key(k) => {
                                let result = self.next().expect("Expected more events after a Key event");
                                object.insert(k, itry!(result));
                            }
                            _ => unreachable!(),
                        }
                    }
                    Some(Ok(Json::Object(object)))
                }
                Event::StartArray => {
                    let mut array = vec![];
                    while let Some(result) = self.next() {
                        array.push(itry!(result));
                    }
                    Some(Ok(Json::Array(array)))
                }
                Event::Null => Some(Ok(Json::Null)),
                Event::Boolean(v) => Some(Ok(Json::Boolean(v))),
                Event::String(v) => Some(Ok(Json::String(v))),
                Event::Number(v) => Some(Ok(Json::F64(v))),
                Event::Key(k) => panic!("Unexpected Key event: {}", k),
            }
        }
    }
}

pub trait Builder where Self: Sized + EventIterator {

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

impl<T> Builder for T where T: Sized + EventIterator {}

pub fn decode<T: Decodable>(json: Json) -> result::Result<T, json::DecoderError> {
    let mut decoder = json::Decoder::new(json);
    Decodable::decode(&mut decoder)
}
