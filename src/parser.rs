use std::io::Read;

use crate::lexer::{Lexer, Lexeme};
use crate::errors::{Error, Result};


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

#[derive(Debug)]
enum State {
    Closed,
    Value,
    ArrayOpen,
    ObjectOpen,
    Colon,
    Comma,
}

#[derive(PartialEq)]
enum Container {
    Object,
    Array,
}

struct ParserState {
    state: State,
    stack: Vec<Container>,
}

impl ParserState {

    #[inline(always)]
    fn process_value<'a>(&mut self, lexeme: Lexeme<'a>) -> Result<Event<'a>> {
        match &lexeme {
            &Lexeme::OBracket => self.stack.push(Container::Array),
            &Lexeme::OBrace => self.stack.push(Container::Object),
            _ => (),
        };
        let result = match lexeme {
            Lexeme::OBracket => Event::StartArray,
            Lexeme::OBrace => Event::StartMap,
            Lexeme::String(s) => Event::String(s),
            Lexeme::Number(n) => Event::Number(n),
            Lexeme::Null => Event::Null,
            Lexeme::Boolean(b) => Event::Boolean(b),
            _ => unreachable!(),
        };

        self.state = if self.stack.is_empty() {
            State::Closed
        } else if lexeme == Lexeme::OBracket {
            State::ArrayOpen
        } else if lexeme == Lexeme::OBrace {
            State::ObjectOpen
        } else {
            State::Comma
        };

        Ok(result)
    }

    #[inline(always)]
    fn process_closing<'a>(&mut self, expected: Container) -> Result<Event<'a>> {
        match self.stack.pop() {
            Some(ref value) if *value == expected => {
                self.state = if self.stack.is_empty() {
                    State::Closed
                } else {
                    State::Comma
                };
                Ok(match expected {
                    Container::Array => Event::EndArray,
                    Container::Object => Event::EndMap,
                })
            }
            _ => Err(Error::Unmatched),
        }
    }

    #[inline(always)]
    fn process_key<'a>(&mut self, lexeme: Lexeme<'a>) -> Result<Event<'a>> {
        self.state = State::Colon;
        match lexeme {
            Lexeme::String(s) => Ok(Event::Key(s)),
            _ => Err(Error::Unexpected),
        }
    }

}

pub struct Parser<T: Read> {
    lexer: Lexer<T>,
    state: ParserState,
}

impl<T: Read> Lexer<T> {
    #[inline]
    pub fn consume(&mut self) -> Result<Lexeme> {
        self.next().unwrap_or(Err(Error::MoreLexemes))
    }
}

impl<T: Read> Parser<T> {

    pub fn new(f: T) -> Parser<T> {
        Parser {
            lexer: Lexer::new(f),
            state: ParserState {
                stack: vec![],
                state: State::Value,
            },
        }
    }

    pub fn next<'a>(&'a mut self) -> Option<Result<Event<'a>>> {
        let event = match self.state.state {
            State::Closed => {
                match self.lexer.next() {
                    Some(Err(Error::IO(..))) | None => return None,
                    Some(..) => Err(Error::AdditionalData),
                }
            }
            State::Value => {
                let lexeme = itry!(self.lexer.consume());
                match lexeme {
                    Lexeme::Comma | Lexeme::Colon | Lexeme::CBrace | Lexeme::CBracket => Err(Error::Unexpected),
                    _ => self.state.process_value(lexeme),
                }
            }
            State::ArrayOpen => {
                let lexeme = itry!(self.lexer.consume());
                match lexeme {
                    Lexeme::Comma | Lexeme::Colon | Lexeme::CBrace => Err(Error::Unexpected),
                    Lexeme::CBracket => self.state.process_closing(Container::Array),
                    _ => self.state.process_value(lexeme),
                }
            }
            State::ObjectOpen => {
                let lexeme = itry!(self.lexer.consume());
                match lexeme {
                    Lexeme::CBrace => self.state.process_closing(Container::Object),
                    Lexeme::String(_) => self.state.process_key(lexeme),
                    _ => Err(Error::Unexpected)
                }
            }
            State::Colon => {
                match itry!(self.lexer.consume()) {
                    Lexeme::Colon => {
                        let lexeme = itry!(self.lexer.consume());
                        self.state.process_value(lexeme)
                    }
                    _ => Err(Error::Unexpected),
                }
            }
            State::Comma => {
                match itry!(self.lexer.consume()) {
                    Lexeme::Comma => {
                        let lexeme = itry!(self.lexer.consume());
                        match *self.state.stack.last().unwrap() {
                            Container::Array => self.state.process_value(lexeme),
                            Container::Object => self.state.process_key(lexeme),
                        }
                    }
                    Lexeme::CBracket => self.state.process_closing(Container::Array),
                    Lexeme::CBrace => self.state.process_closing(Container::Object),
                    _ => Err(Error::Unexpected),
                }
            }
        };
        Some(event)
    }
}
