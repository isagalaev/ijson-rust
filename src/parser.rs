use std::io::Read;

use ::lexer::{Lexer, Lexeme};
use ::errors::{Error, Result};


#[derive(Debug)]
#[derive(PartialEq)]
pub enum Event<'a> {
    Null,
    Boolean(bool),
    String(&'a str),
    Key(&'a str),
    Number(f64),
    StartArray,
    EndArray,
    StartMap,
    EndMap,
}

impl<'a> From<Lexeme<'a>> for Event<'a> {
    #[inline]
    fn from(lexeme: Lexeme<'a>) -> Self {
        match lexeme {
            Lexeme::OBracket => Event::StartArray,
            Lexeme::OBrace => Event::StartMap,
            Lexeme::CBracket => Event::EndArray,
            Lexeme::CBrace => Event::EndMap,
            Lexeme::String(s) => Event::String(s),
            Lexeme::Number(n) => Event::Number(n),
            Lexeme::Null => Event::Null,
            Lexeme::Boolean(b) => Event::Boolean(b),
            Lexeme::Comma | Lexeme::Colon => unreachable!(),
        }
    }
}

#[derive(Debug)]
enum State {
    Closed,
    Event(bool),
    Key(bool),
    Colon,
    Comma,
}

#[derive(PartialEq)]
enum Container {
    Object,
    Array,
}

pub struct Parser<T: Read> {
    lexer: Lexer<T>,
    stack: Vec<Container>,
    state: State,
}


macro_rules! consume_lexeme {
    ($parser: expr) => {
        itry!($parser.lexer.next().unwrap_or(Err(Error::MoreLexemes)))
    }
}

impl<T: Read> Parser<T> {

    pub fn new(f: T) -> Parser<T> {
        Parser {
            lexer: Lexer::new(f),
            stack: vec![],
            state: State::Event(false),
        }
    }

    pub fn next<'a>(&'a mut self) -> Option<Result<Event<'a>>> {
        loop {
            match self.state {
                State::Closed => {
                    return match self.lexer.next() {
                        Some(Err(Error::IO(..))) | None => None,
                        Some(..) => Some(Err(Error::AdditionalData)),
                    }
                }
                State::Event(can_close) => {
                    let lexeme = consume_lexeme!(self);

                    match &lexeme {
                        &Lexeme::CBracket | &Lexeme::CBrace if !can_close => return Some(Err(Error::Unexpected)),
                        &Lexeme::OBracket => self.stack.push(Container::Array),
                        &Lexeme::OBrace => self.stack.push(Container::Object),
                        &Lexeme::CBracket | &Lexeme::CBrace => {
                            let expected = if Lexeme::CBracket == lexeme { Container::Array } else { Container::Object };
                            match self.stack.pop() {
                                Some(ref value) if *value == expected => (),
                                _ => return Some(Err(Error::Unmatched)),
                            }
                        }
                        _ => ()
                    };

                    self.state = if self.stack.is_empty() {
                        State::Closed
                    } else if lexeme == Lexeme::OBracket {
                        State::Event(true)
                    } else if lexeme == Lexeme::OBrace {
                        State::Key(true)
                    } else {
                        State::Comma
                    };

                    return Some(Ok(Event::from(lexeme)))
                }
                State::Key(can_close) => {
                    if itry!(self.lexer.cbrace_next()) {
                        if !can_close {
                            return Some(Err(Error::Unexpected))
                        }
                        self.state = State::Event(true);
                        continue;
                    }
                    return Some(match consume_lexeme!(self) {
                        Lexeme::String(s) => {
                            self.state = State::Colon;
                            Ok(Event::Key(s))
                        }
                        _ => Err(Error::Unexpected)
                    })
                }
                State::Colon => {
                    match consume_lexeme!(self) {
                        Lexeme::Colon => self.state = State::Event(false),
                        _ => return Some(Err(Error::Unexpected)),
                    }
                }
                State::Comma => {
                    if itry!(self.lexer.cbracket_next()) || itry!(self.lexer.cbrace_next()) {
                        self.state = State::Event(true);
                        continue;
                    }
                    match consume_lexeme!(self) {
                        Lexeme::Comma => {
                            self.state = if *self.stack.last().unwrap() == Container::Array {
                                State::Event(false)
                            } else {
                                State::Key(false)
                            }
                        },
                        _ => return Some(Err(Error::Unexpected)),
                    }
                }
            }
        }
    }
}
