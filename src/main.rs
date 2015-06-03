extern crate ijson_rust;

use std::fs::File;

use ijson_rust::parser;


fn main() {
    let f = Box::new(File::open("test.json").unwrap());
    for event in parser::basic_parse(f) {
        match event {
            parser::Event::String(s) => println!("String({})", s),
            parser::Event::Key(s) => println!("Key({})", s),
            _ => println!("{:?}", event),
        }
    }
}
