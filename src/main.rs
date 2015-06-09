extern crate ijson_rust;
extern crate time;

use std::fs::File;

use ijson_rust::parser;
use time::Duration;


fn main() {
    let f = Box::new(File::open("load.json").unwrap());
    let p = parser::basic_parse(f);
    let mut count = 0;

    println!("{}", Duration::span(|| {
        for event in p {
            if let parser::Event::String(s) = event {
                if count == 0 {
                    println!("{}", s);
                }
                count +=1
            }
        }
    }));

    println!("{}", count);
}
