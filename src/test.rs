use std::fs::File;

use super::parser;


#[test]
fn test_basic_parse() {
    let f = Box::new(File::open("test.json").unwrap());
    assert_eq!(parser::basic_parse(f).count(), 44);
}
