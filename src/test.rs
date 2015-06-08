use std::fs::File;

use super::parser::{basic_parse, parse, Event};


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
fn test_basic_parse() {
    let f = Box::new(File::open("test.json").unwrap());
    let events: Vec<Event> = basic_parse(f).collect();
    assert_eq!(events, reference_events());
}

#[test]
fn prefixes() {
    let f = Box::new(File::open("test.json").unwrap());
    let reference = [
        "",
        "",
        "docs",
        "docs.item",
        "docs.item",
        "docs.item.null",
        "docs.item",
        "docs.item.boolean",
        "docs.item",
        "docs.item.true",
        "docs.item",
        "docs.item.integer",
        "docs.item",
        "docs.item.double",
        "docs.item",
        "docs.item.exponent",
        "docs.item",
        "docs.item.long",
        "docs.item",
        "docs.item.string",
        "docs.item",
        "docs.item",
        "docs.item",
        "docs.item.meta",
        "docs.item.meta.item",
        "docs.item.meta.item.item",
        "docs.item.meta.item",
        "docs.item.meta.item",
        "docs.item.meta.item",
        "docs.item.meta",
        "docs.item",
        "docs.item",
        "docs.item",
        "docs.item.meta",
        "docs.item.meta",
        "docs.item.meta.key",
        "docs.item.meta",
        "docs.item",
        "docs.item",
        "docs.item",
        "docs.item.meta",
        "docs.item",
        "docs",
        "",
    ];
    assert!(
        parse(f).map(|(p, _)| p).zip(reference.iter()).all(|(p, &r)| p == r)
    )
}
