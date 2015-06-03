extern crate ijson_rust;

use std::fs::File;

use ijson_rust::parser;


fn main() {
    let f = Box::new(File::open("test.json").unwrap());
    for (prefix, event) in parser::parse(f) {
        println!("{}: {:?}", prefix, event);
    }
}
