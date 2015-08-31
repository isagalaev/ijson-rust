use std::io;

use ::errors::{Error, Result, ResultIterator};


const BUFSIZE: usize = 64 * 1024;


fn is_whitespace(value: u8) -> bool {
    match value {
        9 | 10 | 13 | 32 => true,
        _ => false,
    }
}

fn is_scalar(value: u8) -> bool {
    match value {
        b'a' ... b'z' | b'0' ... b'9' |
        b'E' |  b'.' | b'+' | b'-' => true,
        _ => false,
    }
}

#[derive(Debug, PartialEq)]
pub enum Lexeme {
    String(String),
    Scalar(String),
    OBrace,
    CBrace,
    OBracket,
    CBracket,
    Comma,
    Colon,
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
    type Item = Result<Lexeme>;

    fn next(&mut self) -> Option<Self::Item> {
        while match itry!(self.ensure_buffer()) {
            Buffer::Empty => return None,
            _ => is_whitespace(self.buf[self.pos]),
        } {
            self.pos += 1;
        }

        Some(Ok(if self.buf[self.pos] == b'"' {
            let mut result = vec![];
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
            Lexeme::String(itry!(String::from_utf8(result)))
        } else if !is_scalar(self.buf[self.pos]) {
            let ch = self.buf[self.pos];
            self.pos += 1;
            match ch {
                b'{' => Lexeme::OBrace,
                b'}' => Lexeme::CBrace,
                b'[' => Lexeme::OBracket,
                b']' => Lexeme::CBracket,
                b',' => Lexeme::Comma,
                b':' => Lexeme::Colon,
                _ => return Some(Err(Error::Unknown(ch.to_string()))),
            }
        } else {
            let mut result = vec![];
            loop {
                let start = self.pos;
                while self.pos < self.len && is_scalar(self.buf[self.pos]) {
                    self.pos += 1;
                }
                result.extend(self.buf[start..self.pos].iter().cloned());
                match itry!(self.ensure_buffer()) {
                    Buffer::Reset => (), // continue
                    _ => break,
                }
            }
            Lexeme::Scalar(itry!(String::from_utf8(result)))
        }))
    }
}
