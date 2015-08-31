use std::{io, str, string, error, fmt, result};

use ::lexer::Lexeme;


#[macro_export]
macro_rules! itry {
    ($x: expr) => {
        match $x {
            Err(e) => return Some(Err(From::from(e))),
            Ok(v) => v,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Unterminated,
    IO(io::Error),
    Unknown(String),
    Unexpected(Lexeme),
    Utf8(string::FromUtf8Error),
    Utf8s(str::Utf8Error),
    Escape(String),
    MoreLexemes,
    Unmatched(Lexeme),
    AdditionalData,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        match *self {
            Error::Unterminated => write!(f, "{}", self),
            Error::IO(_) => write!(f, "I/O Error: {}", self),
            Error::Unknown(ref s) => write!(f, "Unexpected lexeme: '{}'", s),
            Error::Unexpected(ref s) => write!(f, "Unexpected lexeme: '{:?}'", s),
            Error::Utf8(ref e) => write!(f, "UTF8 Error: {}", e),
            Error::Utf8s(ref e) => write!(f, "UTF8 Error: {}", e),
            Error::Escape(ref s) => write!(f, "Malformed escape: '{}'", s),
            Error::MoreLexemes => write!(f, "More lexemes expected"),
            Error::Unmatched(ref s) => write!(f, "Unmatched container terminator: {:?}", s),
            Error::AdditionalData => write!(f, "Additional data in the source stream after parsed value"),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Unterminated => "unterminated string",
            Error::IO(ref e) => e.description(),
            Error::Unknown(..) => "unknown lexeme",
            Error::Unexpected(..) => "unexpected lexeme",
            Error::Utf8(ref e) => e.description(),
            Error::Utf8s(ref e) => e.description(),
            Error::Escape(..) => "malformed escape",
            Error::MoreLexemes => "more lexemes expected",
            Error::Unmatched(..) => "unmatched container terminator",
            Error::AdditionalData => "additional data",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Unterminated => None,
            Error::IO(ref e) => Some(e),
            Error::Utf8(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IO(e)
    }
}

impl From<string::FromUtf8Error> for Error {
    fn from(e: string::FromUtf8Error) -> Self {
        Error::Utf8(e)
    }
}

impl From<str::Utf8Error> for Error {
    fn from(e: str::Utf8Error) -> Self {
        Error::Utf8s(e)
    }
}

pub type Result<T> = result::Result<T, Error>;

pub struct ResultIterator<I: Iterator> {
    iterator: I,
    errored: bool,
}

impl<I: Iterator> ResultIterator<I> {
    pub fn new(iterator: I) -> ResultIterator<I> {
        ResultIterator {
            iterator: iterator,
            errored: false,
        }
    }
}

impl<T, I: Iterator<Item=Result<T>>> Iterator for ResultIterator<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.errored {
            return None
        }
        let value = self.iterator.next();
        if let Some(Err(..)) = value {
            self.errored = true
        }
        value
    }
}
