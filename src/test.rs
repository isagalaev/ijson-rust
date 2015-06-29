use std::fs::File;
use std::collections::HashMap;

use super::parser::{Parser, Event};
use super::builder::{Builder, Value};


fn reference_events() -> Vec<Event> {
    vec![
    Event::StartMap,
        Event::Key("docs".to_string()),
        Event::StartArray,
            Event::StartMap,
                Event::Key("null".to_string()),
                Event::Null,
                Event::Key("boolean".to_string()),
                Event::Boolean(false),
                Event::Key("true".to_string()),
                Event::Boolean(true),
                Event::Key("integer".to_string()),
                Event::Number(0f64),
                Event::Key("double".to_string()),
                Event::Number(0.5f64),
                Event::Key("exponent".to_string()),
                Event::Number(100f64),
                Event::Key("long".to_string()),
                Event::Number(10000000000f64),
                Event::Key("string".to_string()),
                Event::String("строка - тест".to_string()),
            Event::EndMap,
            Event::StartMap,
                Event::Key("meta".to_string()),
                Event::StartArray,
                    Event::StartArray,
                        Event::Number(1f64),
                    Event::EndArray,
                    Event::StartMap,
                    Event::EndMap,
                Event::EndArray,
            Event::EndMap,
            Event::StartMap,
                Event::Key("meta".to_string()),
                Event::StartMap,
                    Event::Key("key".to_string()),
                    Event::String("value".to_string()),
                Event::EndMap,
            Event::EndMap,
            Event::StartMap,
                Event::Key("meta".to_string()),
                Event::Null,
            Event::EndMap,
        Event::EndArray,
    Event::EndMap,
    ]
}


#[test]
fn parser() {
    let f = Box::new(File::open("test.json").unwrap());
    let events: Vec<Event> = Parser::new(f).collect();
    assert_eq!(events, reference_events());
}

#[test]
fn prefixes() {
    let f = Box::new(File::open("test.json").unwrap());
    let full: Vec<_> = Parser::new(f).collect();
    let f = Box::new(File::open("test.json").unwrap());
    let result: Vec<_> = Parser::new(f).prefix("").collect();
    assert_eq!(result, full);

    let f = Box::new(File::open("test.json").unwrap());
    let result: Vec<_> = Parser::new(f).prefix("docs.item.meta.item").collect();
    assert_eq!(result, vec![
        Event::StartArray,
        Event::Number(1f64),
        Event::EndArray,
        Event::StartMap,
        Event::EndMap,
    ]);
}

#[test]
fn items() {
    let f = Box::new(File::open("test.json").unwrap());
    let result: Vec<_> = Parser::new(f).items().collect();
    assert_eq!(result.len(), 1);

    let f = Box::new(File::open("test.json").unwrap());
    let items = Parser::new(f).prefix("docs.item.meta.item").items();
    let result: Vec<_> = items.collect();
    assert_eq!(result, vec![
        Value::Array(vec![Value::Number(1f64)]),
        Value::Map(HashMap::new()),
    ]);
}
