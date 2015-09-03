use std::io::Read;
use std::iter::Peekable;

use ::lexer::{Lexer, Lexeme};
use ::errors::{Error, Result, ResultIterator};


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

impl From<Lexeme> for Event {
    fn from(lexeme: Lexeme) -> Self {
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

pub struct Parser<T: Read> {
    lexer: Peekable<ResultIterator<Lexer<T>>>,
    stack: Vec<Lexeme>,
    state: State,
}

impl<T: Read> Parser<T> {

    pub fn new(f: T) -> ResultIterator<Parser<T>> {
        ResultIterator::new(Parser {
            lexer: Lexer::new(f).peekable(),
            stack: vec![],
            state: State::Event(false),
        })
    }

    fn consume_lexeme(&mut self) -> Result<Lexeme> {
        self.lexer.next().unwrap_or(Err(Error::MoreLexemes))
    }

}

impl<T: Read> Iterator for Parser<T> {
    type Item = Result<Event>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.state {
                State::Closed => {
                    return match self.lexer.peek() {
                        Some(&Err(Error::IO(..))) | None => None,
                        Some(..) => Some(Err(Error::AdditionalData)),
                    }
                }
                State::Event(can_close) => {
                    let lexeme = itry!(self.consume_lexeme());

                    match &lexeme {
                        &Lexeme::CBracket | &Lexeme::CBrace if !can_close => return Some(Err(Error::Unexpected(lexeme))),
                        &Lexeme::OBracket => self.stack.push(Lexeme::OBracket),
                        &Lexeme::OBrace => self.stack.push(Lexeme::OBrace),
                        &Lexeme::CBracket | &Lexeme::CBrace => {
                            let expected = if Lexeme::CBracket == lexeme { Lexeme::OBracket } else { Lexeme::OBrace };
                            match self.stack.pop() {
                                Some(ref value) if *value == expected => (),
                                _ => return Some(Err(Error::Unmatched(lexeme))),
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
                    if let Some(&Ok(Lexeme::CBrace)) = self.lexer.peek() {
                        if !can_close {
                            return Some(Err(Error::Unexpected(Lexeme::CBrace)))
                        }
                        self.state = State::Event(true);
                        continue;
                    }
                    return Some(match itry!(self.consume_lexeme()) {
                        Lexeme::String(s) => {
                            self.state = State::Colon;
                            Ok(Event::Key(s))
                        }
                        lexeme => Err(Error::Unexpected(lexeme))
                    })
                }
                State::Colon => {
                    match itry!(self.consume_lexeme()) {
                        Lexeme::Colon => self.state = State::Event(false),
                        lexeme => return Some(Err(Error::Unexpected(lexeme))),
                    }
                }
                State::Comma => {
                    match self.lexer.peek() {
                        Some(&Ok(Lexeme::CBrace)) | Some(&Ok(Lexeme::CBracket)) => {
                            self.state = State::Event(true);
                            continue
                        }
                        _ => (),
                    }
                    match itry!(self.consume_lexeme()) {
                        Lexeme::Comma => {
                            self.state = if self.stack[self.stack.len() - 1] == Lexeme::OBracket {
                                State::Event(false)
                            } else {
                                State::Key(false)
                            }
                        },
                        lexeme => return Some(Err(Error::Unexpected(lexeme))),
                    }
                }
            }
        }
    }
}
