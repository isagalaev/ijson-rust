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

fn main() {
    for chunk in lexer("test.json") {
        println!("{}", str::from_utf8(&chunk).unwrap());
    }
}
