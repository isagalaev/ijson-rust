use std::{io, str, char};

use ::errors::{Error, Result};


const BUFSIZE: usize = 4 * 1024;


#[inline(always)]
fn is_whitespace(value: u8) -> bool {
    match value {
        9 | 10 | 13 | 32 => true,
        _ => false,
    }
}

#[inline(always)]
fn is_number(value: u8) -> bool {
    match value {
        b'+' | b'-' | b'.' | b'0' ... b'9' | b'E' | b'e' => true,
        _ => false,
    }
}

#[derive(Debug, PartialEq)]
pub enum Lexeme {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
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
    tmp: Vec<u8>,
    len: usize,
    pos: usize,
    f: T,
}

impl<T: io::Read> Lexer<T> {

    pub fn new(f: T) -> Lexer<T> {
        Lexer {
            buf: [0; BUFSIZE],
            tmp: Vec::with_capacity(BUFSIZE),
            len: 0,
            pos: 0,
            f: f,
        }
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

    fn hexdecode(&mut self) -> Result<char> {
        let mut value = 0;
        for _ in 0..4 {
            if let Buffer::Empty = try!(self.ensure_buffer()) {
                return Err(Error::Escape(vec![]))
            }
            match (self.buf[self.pos] as char).to_digit(16) {
                None => return Err(Error::Escape(vec![])),
                Some(d) => value = value * 16 + d,
            }
            self.pos += 1;
        }
        char::from_u32(value).map(Ok).unwrap_or(Err(Error::Escape(vec![])))
    }

    fn parse_escape(&mut self) -> Result<char> {
        self.pos += 1; // swallow \
        if let Buffer::Empty = try!(self.ensure_buffer()) {
            return Err(Error::Escape(self.buf[self.pos - 1..].to_vec()))
        }
        let escape = self.buf[self.pos];
        self.pos += 1; // move past the escape symbol
        Ok(match escape {
            b'u' => try!(self.hexdecode()),
            b'b' => '\x08',
            b'f' => '\x0c',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b @ b'"' | b @ b'\\' => b as char,
            c => return Err(Error::Escape(vec![c])),
        })
    }

    fn consume_string(&mut self) -> Result<String> {
        let mut result = Vec::with_capacity(BUFSIZE);
        self.pos += 1;
        loop {
            let start = self.pos;
            while self.pos < self.len && !(self.buf[self.pos] == b'"' || self.buf[self.pos] == b'\\') {
                self.pos += 1;
            }
            result.extend(&self.buf[start..self.pos]);
            match try!(self.ensure_buffer()) {
                Buffer::Empty => return Err(Error::Unterminated),
                Buffer::Reset => (), // continue
                Buffer::Within => {  // " or \
                    if self.buf[self.pos] == b'"' {
                        break
                    }
                    // The ugly bit: parse_escape returns a char and we have
                    // to encode it into utf8 to push into result. This is extra
                    // work and relies on an unstable feature. It would've been
                    // better for parse_escape to produce a unicode byte
                    // sequence directly, but I don't want to encode into utf-8
                    // manually (yet).
                    let ch = try!(self.parse_escape());
                    let mut bytebuf = [0u8; 4];
                    let size = ch.encode_utf8(&mut bytebuf).unwrap();
                    result.extend(&bytebuf[0..size]);
                }
            }
        }
        self.pos += 1;
        Ok(try!(String::from_utf8(result)))
    }

    fn check_word(&mut self, expected: &[u8]) -> Result<()> {
        let mut iter = expected.iter();
        while let Some(byte) = iter.next() {
            if let Buffer::Empty = try!(self.ensure_buffer()) {
                return Err(Error::Unknown(b"".to_vec()))
            }
            if self.buf[self.pos] != *byte {
                return Err(Error::Unknown(self.buf[self.pos..self.pos + 1].to_vec()))
            }
            self.pos += 1;
        }
        Ok(())
    }

    fn consume_number(&mut self) -> Result<f64> {
        let mut start;
        self.tmp.truncate(0);
        loop {
            start = self.pos;
            while self.pos < self.len && is_number(self.buf[self.pos]) {
                self.pos += 1;
            }
            if self.pos < self.len && self.tmp.is_empty() {
                break;
            }
            self.tmp.extend(self.buf[start..self.pos].iter().cloned());
            match try!(self.ensure_buffer()) {
                Buffer::Reset => (), // continue
                _ => break,
            }
        }
        let buffer = if self.tmp.is_empty() {
            &self.buf[start..self.pos]
        } else {
            &self.tmp[..]
        };
        let s = unsafe { str::from_utf8_unchecked(buffer) };
        Ok(try!(s.parse().map_err(|_| Error::Unknown(buffer.to_owned()))))
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

        Some(Ok(match self.buf[self.pos] {
            b'"' => Lexeme::String(itry!(self.consume_string())),
            b't' => {
                itry!(self.check_word(b"true"));
                Lexeme::Boolean(true)
            }
            b'f' => {
                itry!(self.check_word(b"false"));
                Lexeme::Boolean(false)
            }
            b'n' => {
                itry!(self.check_word(b"null"));
                Lexeme::Null
            }
            b'+' | b'-' | b'.' | b'0' ... b'9' => {
                Lexeme::Number(itry!(self.consume_number()))
            }
            byte => {
                self.pos += 1;
                match byte {
                    b'{' => Lexeme::OBrace,
                    b'}' => Lexeme::CBrace,
                    b'[' => Lexeme::OBracket,
                    b']' => Lexeme::CBracket,
                    b',' => Lexeme::Comma,
                    b':' => Lexeme::Colon,
                    _ => return Some(Err(Error::Unknown(vec![byte]))),
                }
            }
        }))
    }
}
