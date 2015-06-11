extern crate ijson_rust;
extern crate time;

use std::fs::File;
use std::env;

use ijson_rust::parser;
use time::Duration;


fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() < 2 {
        panic!("Provide filename")
    }
    let f = Box::new(File::open(&args[1]).unwrap());
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
