use std::fs::File;
use std::io::Read;
use std::str;
use std::iter::Peekable;

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

fn lexer(f: Box<Read>) -> Lexer {
    Lexer {
        buf: [0; BUFSIZE],
        len: 0,
        pos: 0,
        f: f,
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
    Event(bool),
    Key(bool),
    Colon,
    Comma,
}

struct Parser {
    lexer: Peekable<Lexer>,
    stack: Vec<u8>,
    state: State,
}

impl Parser {

    fn consume_lexeme(&mut self) -> Vec<u8> {
        self.lexer.next().expect("More lexemes expected")
    }

    fn check_lexeme(&mut self, lexemes: &[&[u8]]) -> bool {
        match self.lexer.peek() {
            None => false,
            Some(next) => lexemes.iter().any(|l| *l == &next[..]),
        }
    }

    fn process_event(&self, lexeme: &[u8]) -> Event {
        match lexeme {
            b"null" => Event::Null,
            b"true" => Event::Boolean(true),
            b"false" => Event::Boolean(false),
            b"[" => Event::StartArray,
            b"{" => Event::StartMap,
            b"]" => Event::EndArray,
            b"}" => Event::EndMap,
            _ if lexeme[0] == b'"' => Event::String(str::from_utf8(lexeme).unwrap().to_string()),
            _ => Event::Number(
                str::from_utf8(lexeme).unwrap()
                .parse().ok()
                .expect(&format!("Unexpected lexeme {:?}", lexeme))
            )
        }
    }

    #[inline]
    fn assert_top_eq(&mut self, actual: u8) {
        let expected = if actual == b']' { b'[' } else { b'{' };
        match self.stack.pop() {
            Some(value) if value == expected => (),
            _ => panic!("Unmatched {}", actual as char)
        }
    }
}

impl Iterator for Parser {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        loop {
            match self.state {
                State::Closed => {
                    match self.lexer.peek() {
                        Some(_) => panic!("Additional data"),
                        None => return None,
                    }
                }
                State::Event(can_close) => {
                    let lexeme = self.consume_lexeme();

                    match &lexeme[..] {
                        b"]" | b"}" if !can_close => panic!("Unexpected lexeme"),
                        b"[" | b"{" => self.stack.push(lexeme[0]),
                        b"]" | b"}" => self.assert_top_eq(lexeme[0]),
                        _ => ()
                    };

                    self.state = if self.stack.len() == 0 {
                        State::Closed
                    } else if lexeme == b"[" {
                        State::Event(true)
                    } else if lexeme == b"{" {
                        State::Key(true)
                    } else {
                        State::Comma
                    };

                    return Some(self.process_event(&lexeme))
                }
                State::Key(can_close) => {
                    if self.check_lexeme(&[b"}"]) {
                        if !can_close {
                            panic!("Unexpected lexeme")
                        }
                        self.state = State::Event(true);
                        continue;
                    }
                    let lexeme = self.consume_lexeme();
                    if lexeme[0] != b'"' {
                        panic!("Unexpected lexeme")
                    }
                    self.state = State::Colon;
                    return Some(Event::Key(str::from_utf8(&lexeme[..]).unwrap().to_string()));
                }
                State::Colon => {
                    if self.consume_lexeme() != b":" {
                        panic!("Unexpected lexeme")
                    }
                    self.state = State::Event(false);
                }
                State::Comma => {
                    if self.check_lexeme(&[b"]", b"}"]) {
                        self.state = State::Event(true);
                        continue;
                    }
                    if self.consume_lexeme() != b"," {
                        panic!("Unexpected lexeme");
                    }
                    self.state = if self.stack[self.stack.len() - 1] == b'[' {
                        State::Event(false)
                    } else {
                        State::Key(false)
                    };
                }
            }
        }
    }
}

fn basic_parse(f: Box<Read>) -> Parser {
    Parser {
        lexer: lexer(f).peekable(),
        stack: vec![],
        state: State::Event(false),
    }
}

fn main() {
    let f = Box::new(File::open("test.json").unwrap());
    for event in basic_parse(f) {
        println!("{:?}", event);
    }
}
