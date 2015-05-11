#![feature(convert)]
extern crate ijson_rust;

use std::fs::File;
use std::io::Read;
use std::str;

const BUFSIZE: usize = 10;

fn is_whitespace(value: u8) -> bool {
    match value {
        9 | 10 | 13 | 32 => true,
        _ => false,
    }
}

fn is_lexeme(value: u8) -> bool {
    match value {
        b'a' ... b'z' | b'0' ... b'9' |
        b'E' |  b'.' | b'+' | b'-' => true,
        _ => false,
    }
}

struct Lexer {
    buf: [u8; BUFSIZE],
    len: usize,
    pos: usize,
    f: Box<Read>,
}

impl Lexer {
    fn ensure_buffer(&mut self) -> bool {
        if self.pos < self.len {
            true
        } else {
            match self.f.read(&mut self.buf) {
                Err(error) => panic!("Error reading stream: {}", error),
                Ok(size) => { self.len = size; self.pos = 0; },
            };
            self.len > 0
        }
    }
}

impl Iterator for Lexer {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Vec<u8>> {
        while self.ensure_buffer() && is_whitespace(self.buf[self.pos]) {
            self.pos += 1;
        }
        if self.len == 0 {
            return None;
        }

        let mut result = vec![];
        if self.buf[self.pos] == b'"' {
            result.push(b'"');
            let mut escaped = false;
            self.pos += 1;
            loop {
                let start = self.pos;
                while self.pos < self.len && (escaped || self.buf[self.pos] != b'"') {
                    escaped = !escaped && self.buf[self.pos] == b'\\';
                    self.pos += 1;
                }
                result.extend(self.buf[start..self.pos].iter().cloned());
                if self.pos < self.len {
                    self.pos += 1;
                    break;
                } else if !self.ensure_buffer() {
                    panic!("Unterminated string");
                }
            }
            result.push(b'"');
        } else if !is_lexeme(self.buf[self.pos]) {
            result.push(self.buf[self.pos]);
            self.pos += 1;
        } else {
            loop {
                let start = self.pos;
                while self.pos < self.len && is_lexeme(self.buf[self.pos]) {
                    self.pos += 1;
                }
                result.extend(self.buf[start..self.pos].iter().cloned());
                if self.pos < self.len || !self.ensure_buffer() {
                    break;
                }
            }
        }
        Some(result)
    }
}

fn lexer(filename: &str) -> Lexer {
    Lexer {
        buf: [0; BUFSIZE],
        len: 0,
        pos: 0,
        f: match File::open(filename) {
            Err(error) => panic!("Can't open {}: {}", filename, error),
            Ok(result) => Box::new(result),
        },
    }
}

#[derive(Debug)]
enum Event {
    Null,
    Boolean(bool),
    String(String),
    Key(String),
    Number(i64),
    StartArray,
    EndArray,
    StartMap,
    EndMap,
}

enum State {
    Event(bool), // bool flags skippable commas
    Key,
    Colon,
}

struct Parser {
    lexer: Lexer, // TODO: iterator of Vec<u8>
    stack: Vec<u8>,
    state: State,
    closed: bool,
}

impl Parser {

    fn next_lexeme(&mut self) -> Vec<u8> {
        self.lexer.next().expect("More lexemes expected")
    }

    fn close(&mut self) {
        self.closed = true;
        match self.lexer.next() {
            None => (),
            Some(_) => panic!("Additional data"),
        };
    }

}

impl Iterator for Parser {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        if self.closed {
            return None;
        }
        loop {
            let lexeme = self.next_lexeme();
            match self.state {
                State::Event(skip_comma) => {
                    let lexeme = if skip_comma && lexeme == b"," {
                        self.next_lexeme()
                    } else {
                        lexeme
                    };
                    let result = if lexeme == b"null" {
                        Event::Null
                    } else if lexeme == b"true" {
                        Event::Boolean(true)
                    } else if lexeme == b"false" {
                        Event::Boolean(false)
                    } else if lexeme == b"[" {
                        self.stack.push(b'[');
                        self.state = State::Event(false);
                        Event::StartArray
                    } else if lexeme == b"]" {
                        if self.stack.len() == 0 || self.stack.pop().unwrap() != b'[' {
                            panic!("Unmatched ]");
                        }
                        Event::EndArray
                    } else if lexeme == b"{" {
                        self.stack.push(b'{');
                        self.state = State::Key;
                        Event::StartMap
                    } else if lexeme == b"}" {
                        if self.stack.len() == 0 || self.stack.pop().unwrap() != b'{' {
                            panic!("Unmatched }");
                        }
                        Event::EndMap
                    } else if lexeme[0] == b'"' {
                        Event::String(str::from_utf8(lexeme.as_slice()).unwrap().to_string())
                    } else {
                        Event::Number(0) // TODO: convert number
                    };
                    if self.stack.len() == 0 {
                        self.close();
                    } else {
                        self.state = State::Event(true);
                    };
                    return Some(result);
                }
                State::Key => {
                    if lexeme[0] != b'"' {
                        panic!("Unexpected lexeme");
                    }
                    self.state = State::Colon;
                    return Some(Event::Key(str::from_utf8(lexeme.as_slice()).ok().unwrap().to_string()));
                }
                State::Colon => {
                    if lexeme != b":" {
                        panic!("Unexpected lexeme");
                    }
                    self.state = State::Event(false);
                }
            };
        }
    }
}

fn basic_parse(filename: &str) -> Parser {
    Parser {
        lexer: lexer(filename),
        stack: vec![],
        state: State::Event(false),
        closed: false,
    }
}

fn main() {
    for event in basic_parse("test.json") {
        println!("{:?}", event);
    }
}
