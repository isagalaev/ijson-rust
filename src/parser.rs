use std::io::Read;
use std::iter::Peekable;
use std::{str, char, error, fmt, result};

use ::lexer;


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
pub enum Error {
    Unexpected(String),
    Utf8(str::Utf8Error),
    Escape(String),
    MoreLexemes,
    Unmatched(char),
    Lexer(lexer::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        match *self {
            Error::Unexpected(ref s) => write!(f, "Unexpected lexeme: '{}'", s),
            Error::Utf8(ref e) => write!(f, "UTF8 Error: {}", e),
            Error::Escape(ref s) => write!(f, "Malformed escape: '{}'", s),
            Error::MoreLexemes => write!(f, "More lexemes expected"),
            Error::Unmatched(ref c) => write!(f, "Unmatched container terminator: {}", c),
            Error::Lexer(ref e) => write!(f, "Lexer error: {}", e),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Unexpected(..) => "unexpected lexeme",
            Error::Utf8(ref e) => e.description(),
            Error::Escape(..) => "malformed escape",
            Error::MoreLexemes => "more lexemes expected",
            Error::Unmatched(..) => "unmatched container terminator",
            Error::Lexer(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Lexer(ref e) => Some(e),
            Error::Utf8(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<str::Utf8Error> for Error {
    fn from(e: str::Utf8Error) -> Self {
        Error::Utf8(e)
    }
}

pub type Result<T> = result::Result<T, Error>;

#[inline]
fn unexpected(lexeme: Vec<u8>) -> Option<Result<Event>> {
    Some(Err(Error::Unexpected(str::from_utf8(&lexeme[..]).unwrap().to_string())))
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

fn unescape(lexeme: &[u8]) -> Result<String> {
    let len = lexeme.len();
    let mut result = String::with_capacity(lexeme.len());

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
    lexer: Peekable<lexer::Lexer<T>>,
    stack: Vec<u8>,
    state: State,
}

impl<T: Read> Parser<T> {

    pub fn new(f: T) -> Parser<T> {
        Parser {
            lexer: lexer::Lexer::new(f).peekable(),
            stack: vec![],
            state: State::Event(false),
        }
    }

    fn consume_lexeme(&mut self) -> Result<Vec<u8>> {
        match self.lexer.next() {
            None => Err(Error::MoreLexemes),
            Some(Err(e)) => Err(Error::Lexer(e)),
            Some(Ok(v)) => Ok(v),
        }
    }

    fn check_lexeme(&mut self, lexemes: &[&[u8]]) -> bool {
        match self.lexer.peek() {
            None | Some(&Err(..)) => false,
            Some(&Ok(ref next)) => {
                lexemes.iter().any(|l| *l == &next[..])
            }
        }
    }

    fn process_event(&self, lexeme: &[u8]) -> Result<Event> {
        Ok(match lexeme {
            b"null" => Event::Null,
            b"true" => Event::Boolean(true),
            b"false" => Event::Boolean(false),
            b"[" => Event::StartArray,
            b"{" => Event::StartMap,
            b"]" => Event::EndArray,
            b"}" => Event::EndMap,
            _ if lexeme[0] == b'"' => Event::String(try!(unescape(trim(lexeme)))),
            _ => {
                let s = try!(str::from_utf8(lexeme));
                Event::Number(try!(s.parse().map_err(|_| Error::Unexpected(str::from_utf8(lexeme).unwrap().to_string()))))
            }
        })
    }

}

impl<T: Read> Iterator for Parser<T> {
    type Item = Result<Event>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.state {
                State::Closed => {
                    match self.lexer.peek() {
                        Some(_) => panic!("Additional data"),
                        None => return None,
                    }
                }
                State::Event(can_close) => {
                    let lexeme = itry!(self.consume_lexeme());

                    match &lexeme[..] {
                        b"]" | b"}" if !can_close => return unexpected(lexeme),
                        b"[" | b"{" => self.stack.push(lexeme[0]),
                        b"]" | b"}" => {
                            let expected = if lexeme[0] == b']' { b'[' } else { b'{' };
                            match self.stack.pop() {
                                Some(value) if value == expected => (),
                                _ => return Some(Err(Error::Unmatched(lexeme[0] as char))),
                            }
                        }
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
                            return unexpected(vec![b'}'])
                        }
                        self.state = State::Event(true);
                        continue;
                    }
                    let lexeme = itry!(self.consume_lexeme());
                    if lexeme[0] != b'"' {
                        return unexpected(lexeme)
                    }
                    self.state = State::Colon;
                    let s = itry!(str::from_utf8(trim(&lexeme)));
                    return Some(Ok(Event::Key(s.to_string())))
                }
                State::Colon => {
                    let lexeme = itry!(self.consume_lexeme());
                    if lexeme != b":" {
                        return unexpected(lexeme)
                    }
                    self.state = State::Event(false);
                }
                State::Comma => {
                    if self.check_lexeme(&[b"]", b"}"]) {
                        self.state = State::Event(true);
                        continue;
                    }
                    let lexeme = itry!(self.consume_lexeme());
                    if lexeme != b"," {
                        return unexpected(lexeme)
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
