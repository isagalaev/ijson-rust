#![feature(convert)]
extern crate ijson_rust;

use std::fs::File;
use std::io::Read;
use std::str;
use std::str::FromStr;

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
    Number(f64),
    StartArray,
    EndArray,
    StartMap,
    EndMap,
}

#[derive(Debug)]
enum State {
    Closed,
    Event,
    Key,
    Colon,
    Comma,
}

struct Parser {
    lexer: Lexer, // TODO: iterator of Vec<u8>
    stack: Vec<u8>,
    state: State,
}

impl Parser {

    fn next_lexeme(&mut self) -> Vec<u8> {
        match self.lexer.next() {
            Some(lexeme) => match self.state {
                State::Closed => panic!("Additional data"),
                _ => lexeme,
            },
            None => match self.state {
                State::Closed => vec![],
                _ => panic!("More lexemes expected"),
            },
        }
    }

    fn process_event(&mut self, lexeme: &Vec<u8>) -> Event {

        let result = if lexeme == b"null" {
            Event::Null
        } else if lexeme == b"true" {
            Event::Boolean(true)
        } else if lexeme == b"false" {
            Event::Boolean(false)
        } else if lexeme[0] == b'"' {
            Event::String(str::from_utf8(lexeme.as_slice()).unwrap().to_string())
        } else if lexeme == b"[" {
            self.stack.push(b'[');
            Event::StartArray
        } else if lexeme == b"{" {
            self.stack.push(b'{');
            Event::StartMap
        } else if lexeme == b"]" {
            if self.stack.len() == 0 || self.stack.pop().unwrap() != b'[' {
                panic!("Unmatched ]");
            }
            Event::EndArray
        } else if lexeme == b"}" {
            if self.stack.len() == 0 || self.stack.pop().unwrap() != b'{' {
                panic!("Unmatched }");
            }
            Event::EndMap
        } else {
            let s = str::from_utf8(lexeme.as_slice()).unwrap();
            let number = match f64::from_str(s) {
                Err(_) => panic!("Unexpected lexeme {:?}", lexeme),
                Ok(result) => result,
            };
            Event::Number(number)
        };

        self.state = if self.stack.len() == 0 {
            State::Closed
        } else if lexeme == b"[" {
            State::Event
        } else if lexeme == b"{" {
            State::Key
        } else {
            State::Comma
        };

        result
    }

}

impl Iterator for Parser {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        loop {
            let lexeme = self.next_lexeme();
            match self.state {
                State::Closed => {
                    return None;
                }
                State::Event => {
                    return Some(self.process_event(&lexeme));
                }
                State::Key => {
                    if lexeme == b"}" {
                        return Some(self.process_event(&lexeme));
                    }
                    if lexeme[0] != b'"' {
                        panic!("Unexpected lexeme");
                    }
                    self.state = State::Colon;
                    return Some(Event::Key(str::from_utf8(lexeme.as_slice()).unwrap().to_string()));
                }
                State::Colon => {
                    if lexeme != b":" {
                        panic!("Unexpected lexeme");
                    }
                    self.state = State::Event;
                }
                State::Comma => {
                    if lexeme == b"]" || lexeme == b"}" {
                        return Some(self.process_event(&lexeme));
                    }
                    if lexeme != b"," {
                        panic!("Unexpected lexeme");
                    }
                    self.state = if self.stack[self.stack.len() - 1] == b'[' {
                        State::Event
                    } else {
                        State::Key
                    };
                }
            }
        }
    }
}

fn basic_parse(filename: &str) -> Parser {
    Parser {
        lexer: lexer(filename),
        stack: vec![],
        state: State::Event,
    }
}

fn main() {
    for event in basic_parse("test.json") {
        println!("{:?}", event);
    }
}
