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

fn is_number(value: u8) -> bool {
    match value {
        b'+' | b'-' | b'.' | b'0' ... b'9' | b'E' | b'e' => true,
        _ => false,
    }
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
    len: usize,
    pos: usize,
    f: T,
}

impl<T: io::Read> Lexer<T> {

    pub fn new(f: T) -> Lexer<T> {
        Lexer {
            buf: [0; BUFSIZE],
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

    fn ensure_at_least(&mut self, min: usize) -> io::Result<Buffer> {
        let remainder = self.len - self.pos;
        if remainder >= min {
            Ok(Buffer::Within)
        } else {
            for i in 0..remainder {
                self.buf[i] = self.buf[self.pos + i]
            }
            self.pos = 0;
            self.f.read(&mut self.buf[remainder..]).and_then(|size| {
                self.len = remainder + size;
                Ok(if self.len >= min { Buffer::Reset } else { Buffer::Empty })
            })
        }
    }

    fn parse_escape(&mut self) -> Result<char> {
        if let Buffer::Empty = try!(self.ensure_at_least(2)) {
            return Err(Error::Escape(self.buf[self.pos..].to_vec()))
        }
        self.pos += 1; // swallow \
        let escape = self.buf[self.pos];
        self.pos += 1; // move past the escape symbol
        Ok(match escape {
            b'u' => {
                if let Buffer::Empty = try!(self.ensure_at_least(4)) {
                    return Err(Error::Escape(self.buf[self.pos..].to_vec()))
                }
                let s = &self.buf[self.pos..self.pos + 4];
                self.pos += 4;
                match hexdecode(s) {
                    None => return Err(Error::Escape(s.to_vec())),
                    Some(ch) => ch,
                }
            }
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
        let mut result = String::with_capacity(BUFSIZE);
        self.pos += 1;
        loop {
            let start = self.pos;
            while self.pos < self.len && !(self.buf[self.pos] == b'"' || self.buf[self.pos] == b'\\') {
                self.pos += 1;
            }
            result.push_str(try!(str::from_utf8(&self.buf[start..self.pos])));
            match try!(self.ensure_buffer()) {
                Buffer::Empty => return Err(Error::Unterminated),
                Buffer::Reset => (), // continue
                Buffer::Within => {  // " or \
                    if self.buf[self.pos] == b'"' {
                        break
                    }
                    result.push(try!(self.parse_escape()))
                }
            }
        }
        self.pos += 1;
        Ok(result)
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

    fn consume_number(&mut self) -> Result<Vec<u8>> {
        let mut result = vec![];
        loop {
            let start = self.pos;
            while self.pos < self.len && is_number(self.buf[self.pos]) {
                self.pos += 1;
            }
            result.extend(self.buf[start..self.pos].iter().cloned());
            match try!(self.ensure_buffer()) {
                Buffer::Reset => (), // continue
                _ => break,
            }
        }
        Ok(result)
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
            Lexeme::String(itry!(self.consume_string()))
        } else if self.buf[self.pos] == b't'     {
            itry!(self.check_word(b"true"));
            Lexeme::Boolean(true)
        } else if self.buf[self.pos] == b'f' {
            itry!(self.check_word(b"false"));
            Lexeme::Boolean(false)
        } else if self.buf[self.pos] == b'n' {
            itry!(self.check_word(b"null"));
                Lexeme::Null
        } else if is_number(self.buf[self.pos]) {
            let buffer = itry!(self.consume_number());
            let s = unsafe { str::from_utf8_unchecked(&buffer[..]) };
            Lexeme::Number(itry!(s.parse().map_err(|_| Error::Unknown(buffer.clone()))))
        } else {
            let ch = self.buf[self.pos];
            self.pos += 1;
            match ch {
                b'{' => Lexeme::OBrace,
                b'}' => Lexeme::CBrace,
                b'[' => Lexeme::OBracket,
                b']' => Lexeme::CBracket,
                b',' => Lexeme::Comma,
                b':' => Lexeme::Colon,
                _ => return Some(Err(Error::Unknown(vec![ch]))),
            }
        }))
    }
}
