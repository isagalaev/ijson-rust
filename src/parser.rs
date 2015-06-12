use std::io::Read;
use std::iter::Peekable;
use std::str;
use std::char;

use super::lexer;


#[derive(Debug)]
#[derive(PartialEq)]
pub enum Event {
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

#[inline]
fn trim(lexeme: &[u8]) -> &[u8] {
    &lexeme[1..lexeme.len() - 1]
}

fn unescape(lexeme: &[u8]) -> String {
    let len = lexeme.len();
    let mut result = String::with_capacity(lexeme.len());

    let mut pos = 0;
    while pos < len {
        let start = pos;
        while pos < len && lexeme[pos] != b'\\' {
            pos += 1;
        }
        result.push_str(str::from_utf8(&lexeme[start..pos]).unwrap());
        if pos < len {
            pos += 1; // safe to do as the lexer makes sure there's at lease one character after \
            result.push(match lexeme[pos] {
                b'u' => {
                    if pos + 4 >= len {
                        panic!("Malformed escape")
                    }
                    let value = lexeme[pos+1..pos+5].iter().fold(0, |acc, &c| acc * 16 + (c as char).to_digit(16).unwrap());
                    pos += 4;
                    char::from_u32(value).unwrap()
                }
                b'b' => '\x08',
                b'f' => '\x0c',
                b'n' => '\n',
                b'r' => '\r',
                b't' => '\t',
                b @ b'"' | b @ b'\\' => b as char,
                _ => panic!("Malformed escape"),
            });
            pos += 1;
        }
    }
    result
}

pub struct Parser {
    lexer: Peekable<lexer::Lexer>,
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
            _ if lexeme[0] == b'"' => Event::String(unescape(trim(lexeme))),
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
                    return Some(Event::Key(str::from_utf8(trim(&lexeme)).unwrap().to_string()));
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

pub fn basic_parse(f: Box<Read>) -> Parser {
    Parser {
        lexer: lexer::lexer(f).peekable(),
        stack: vec![],
        state: State::Event(false),
    }
}

pub struct PrefixedParser {
    path: Vec<String>,
    parser: Parser,
}

impl Iterator for PrefixedParser {
    type Item = (String, Event);

    fn next(&mut self) -> Option<Self::Item> {
        match self.parser.next() {
            None => None,
            Some(event) => {
                match &event {
                    &Event::Key(_) | &Event::EndMap | &Event::EndArray => {
                        self.path.pop();
                    }
                    _ => (),
                }
                let prefix = self.path.connect(".");
                match &event {
                    &Event::Key(ref value) => {
                        self.path.push(value.clone());
                    }
                    &Event::StartMap => {
                        self.path.push("".to_owned())
                    }
                    &Event::StartArray => {
                        self.path.push("item".to_owned());
                    }
                    _ => (),
                }
                Some((prefix, event))
            }
        }
    }
}

pub fn parse(f: Box<Read>) -> PrefixedParser {
    PrefixedParser {
        path: vec![],
        parser: basic_parse(f),
    }
}
