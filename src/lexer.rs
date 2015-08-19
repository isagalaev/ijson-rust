use std::{io, fmt, error};

use ::errors::ResultIterator;


const BUFSIZE: usize = 64 * 1024;


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

#[derive(Debug)]
pub enum Error {
    Unterminated,
    IO(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Error::Unterminated => write!(f, "{}", self),
            Error::IO(_) => write!(f, "I/O Error: {}", self)
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Unterminated => "unterminated string",
            Error::IO(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Unterminated => None,
            Error::IO(ref e) => Some(e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IO(e)
    }
}

enum Buffer {
    Within,
    Reset,
    Empty,
}

pub struct Lexer<T: io::Read> {
    buf: [u8; BUFSIZE],
    len: usize,
    pos: usize,
    f: T,
}

impl<T: io::Read> Lexer<T> {

    pub fn new(f: T) -> ResultIterator<Lexer<T>> {
        ResultIterator::new(Lexer {
            buf: [0; BUFSIZE],
            len: 0,
            pos: 0,
            f: f,
        })
    }

    fn ensure_buffer(&mut self) -> io::Result<Buffer> {
        if self.pos < self.len {
            Ok(Buffer::Within)
        } else {
            self.f.read(&mut self.buf).and_then(|size| {
                self.len = size;
                self.pos = 0;
                Ok(if size > 0 { Buffer::Reset } else { Buffer::Empty })
            })
        }
    }

}

impl<T: io::Read> Iterator for Lexer<T> {
    type Item = Result<Vec<u8>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        while match itry!(self.ensure_buffer()) {
            Buffer::Empty => return None,
            _ => is_whitespace(self.buf[self.pos]),
        } {
            self.pos += 1;
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
                match itry!(self.ensure_buffer()) {
                    Buffer::Empty => return Some(Err(Error::Unterminated)),
                    Buffer::Within => break,
                    Buffer::Reset => (), // continue
                }
            }
            self.pos += 1;
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
                match itry!(self.ensure_buffer()) {
                    Buffer::Reset => (), // continue
                    _ => break,
                }
            }
        }
        Some(Ok(result))
    }
}
