use std::{io, str, error, fmt, result};


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
    Unexpected(String),
    Utf8(str::Utf8Error),
    Escape(String),
    MoreLexemes,
    Unmatched(char),
    AdditionalData,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        match *self {
            Error::Unterminated => write!(f, "{}", self),
            Error::IO(_) => write!(f, "I/O Error: {}", self),
            Error::Unexpected(ref s) => write!(f, "Unexpected lexeme: '{}'", s),
            Error::Utf8(ref e) => write!(f, "UTF8 Error: {}", e),
            Error::Escape(ref s) => write!(f, "Malformed escape: '{}'", s),
            Error::MoreLexemes => write!(f, "More lexemes expected"),
            Error::Unmatched(ref c) => write!(f, "Unmatched container terminator: {}", c),
            Error::AdditionalData => write!(f, "Additional data in the source stream after parsed value"),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Unterminated => "unterminated string",
            Error::IO(ref e) => e.description(),
            Error::Unexpected(..) => "unexpected lexeme",
            Error::Utf8(ref e) => e.description(),
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

impl From<str::Utf8Error> for Error {
    fn from(e: str::Utf8Error) -> Self {
        Error::Utf8(e)
    }
}

pub type Result<T> = result::Result<T, Error>;
