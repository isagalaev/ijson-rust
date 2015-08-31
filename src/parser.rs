use std::io::Read;
use std::iter::Peekable;
use std::{str, char};

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

#[derive(Debug)]
enum State {
    Closed,
    Event(bool),
    Key(bool),
    Colon,
    Comma,
}

#[inline]
fn hexdecode(s: &[u8]) -> Option<char> {
    let mut value = 0;
    for c in s.iter() {
        match (*c as char).to_digit(16) {
            None => return None,
            Some(d) => value = value * 16 + d,
        }
    }
    char::from_u32(value)
}

fn unescape(lexeme_str: String) -> Result<String> {
    let lexeme = lexeme_str.as_bytes();
    let len = lexeme.len();
    let mut result = String::with_capacity(len);
    let mut pos = 0;
    while pos < len {
        let start = pos;
        while pos < len && lexeme[pos] != b'\\' {
            pos += 1;
        }
        result.push_str(try!(str::from_utf8(&lexeme[start..pos])));
        if pos < len {
            pos += 1; // safe to do as the lexer makes sure there's at lease one character after \
            result.push(match lexeme[pos] {
                b'u' => {
                    if pos + 4 >= len {
                        return Err(Error::Escape(str::from_utf8(&lexeme[pos..]).unwrap().to_string()))
                    }
                    let s = &lexeme[pos+1..pos+5];
                    pos += 4;
                    match hexdecode(s) {
                        None => return Err(Error::Escape(str::from_utf8(s).unwrap().to_string())),
                        Some(ch) => ch,
                    }
                }
                b'b' => '\x08',
                b'f' => '\x0c',
                b'n' => '\n',
                b'r' => '\r',
                b't' => '\t',
                b @ b'"' | b @ b'\\' => b as char,
                c => return Err(Error::Escape(str::from_utf8(&[c]).unwrap().to_string())),
            });
            pos += 1;
        }
    }
    Ok(result)
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

    fn process_event(&self, lexeme: Lexeme) -> Result<Event> {
        Ok(match lexeme {
            Lexeme::OBracket => Event::StartArray,
            Lexeme::OBrace => Event::StartMap,
            Lexeme::CBracket => Event::EndArray,
            Lexeme::CBrace => Event::EndMap,
            Lexeme::String(s) => Event::String(try!(unescape(s))),
            Lexeme::Scalar(ref s) if s == "null" => Event::Null,
            Lexeme::Scalar(ref s) if s == "true" => Event::Boolean(true),
            Lexeme::Scalar(ref s) if s == "false" => Event::Boolean(false),
            Lexeme::Scalar(s) => {
                Event::Number(try!(s.parse().map_err(|_| Error::Unknown(s))))
            },
            _ => unreachable!(),
        })
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

                    self.state = if self.stack.len() == 0 {
                        State::Closed
                    } else if lexeme == Lexeme::OBracket {
                        State::Event(true)
                    } else if lexeme == Lexeme::OBrace {
                        State::Key(true)
                    } else {
                        State::Comma
                    };

                    return Some(self.process_event(lexeme))
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
