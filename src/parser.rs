use std::io::Read;
use std::iter::Peekable;
use std::str;
use std::char;

use super::lexer;


#[derive(Debug)]
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

//5:08:01 PM - XMPPwocky: isagalaev: you may want to write something on top of a Reader
//5:08:21 PM - XMPPwocky: specifically something over a Cursor<Vec<u8>>, actually
fn unescape(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        result.push(
            if ch != '\\' {
                ch
            } else {
                match chars.next() {
                    Some('u') => {
                        let value = chars.by_ref().take(4).fold(0, |acc, c| acc * 16 + c.to_digit(16).unwrap());
                        char::from_u32(value).unwrap()
                    }
                    Some('b') => '\x08',
                    Some('f') => '\x0c',
                    Some('n') => '\n',
                    Some('r') => '\r',
                    Some('t') => '\t',
                    Some(ch) => ch,
                    _ => panic!("Malformed escape"),
                }
            }
        )
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
            _ if lexeme[0] == b'"' => Event::String(unescape(str::from_utf8(lexeme).unwrap())),
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

pub fn basic_parse(f: Box<Read>) -> Parser {
    Parser {
        lexer: lexer::lexer(f).peekable(),
        stack: vec![],
        state: State::Event(false),
    }
}
