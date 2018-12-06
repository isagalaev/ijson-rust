use std::{io, char, str};

use crate::errors::{Error, Result};


const BUFSIZE: usize = 4 * 1024;


#[inline(always)]
fn is_whitespace(value: u8) -> bool {
    match value {
        9 | 10 | 13 | 32 => true,
        _ => false,
    }
}

#[derive(Debug, PartialEq)]
pub enum Lexeme<'a> {
    String(&'a str),
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
            if let Buffer::Empty = self.ensure_buffer()? {
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
        if let Buffer::Empty = self.ensure_buffer()? {
            return Err(Error::Escape(self.buf[self.pos - 1..].to_vec()))
        }
        let escape = self.buf[self.pos];
        self.pos += 1; // move past the escape symbol
        Ok(match escape {
            b'u' => self.hexdecode()?,
            b'b' => '\x08',
            b'f' => '\x0c',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b @ b'"' | b @ b'\\' => b as char,
            c => return Err(Error::Escape(vec![c])),
        })
    }

    fn consume_string<'a>(&'a mut self) -> Result<&'a str> {
        let mut in_tmp = false;
        let mut start;
        let mut encode_buffer = [0; 5];
        self.pos += 1;
        loop {
            start = self.pos;
            while self.pos < self.len && !(self.buf[self.pos] == b'"' || self.buf[self.pos] == b'\\') {
                self.pos += 1;
            }
            if self.pos >= self.len || self.buf[self.pos] == b'\\' {
                if !in_tmp {
                    unsafe { self.tmp.set_len(0); }
                    in_tmp = true;
                }
                self.tmp.extend_from_slice(&self.buf[start..self.pos]);
            }
            match self.ensure_buffer()? {
                Buffer::Empty => return Err(Error::Unterminated),
                Buffer::Within if self.buf[self.pos] == b'"' => break,
                Buffer::Within => { // b'\'
                    // The ugly bit: parse_escape returns a char and we have
                    // to encode it into utf8 to push into self.tmp. This is extra
                    // work and relies on an unstable feature. It would've been
                    // better for parse_escape to produce a unicode byte
                    // sequence directly, but I don't want to encode into utf-8
                    // manually (yet).
                    let ch = self.parse_escape()?;
                    let count = ch.encode_utf8(&mut encode_buffer).len();
                    self.tmp.extend(&encode_buffer[..count]);
                }
                _ => (),
            }
        }
        let result = if in_tmp {
            self.tmp.extend_from_slice(&self.buf[start..self.pos]);
            &self.tmp[..]
        } else {
            &self.buf[start..self.pos]
        };
        self.pos += 1;
        Ok(str::from_utf8(result)?)
    }

    fn check_word(&mut self, expected: &[u8]) -> Result<()> {
        let mut iter = expected.iter();
        while let Some(byte) = iter.next() {
            if let Buffer::Empty = self.ensure_buffer()? {
                return Err(Error::Unknown(b"".to_vec()))
            }
            if self.buf[self.pos] != *byte {
                return Err(Error::Unknown(self.buf[self.pos..self.pos + 1].to_vec()))
            }
            self.pos += 1;
        }
        Ok(())
    }

    fn consume_sign(&mut self) -> bool {
        match self.buf[self.pos] {
            b'-' => { self.pos += 1; false }
            b'+' => { self.pos += 1; true }
            _ => true,
        }
    }

    #[inline(always)]
    fn consume_int(&mut self, acc: &mut i64) -> Result<(usize)> {
        let mut count = 0;
        loop {
            if let Buffer::Empty = self.ensure_buffer()? {
                break
            }
            match self.buf[self.pos] {
                byte @ b'0'...b'9' => *acc = *acc * 10 + (byte - b'0') as i64,
                _ => break,
            }
            self.pos += 1;
            count += 1;
        }
        Ok(count)
    }

    fn consume_number(&mut self) -> Result<f64> {
        let sign = self.consume_sign();
        let mut int = 0;
        if self.consume_int(&mut int)? == 0 && (self.pos >= self.len || self.buf[self.pos] != b'.') {
            return Err(Error::Unknown(vec![]))
        }
        let mut pow = 0;
        if self.pos < self.len && self.buf[self.pos] == b'.' {
            self.pos += 1;
            pow -= self.consume_int(&mut int)? as i64;
        }
        if self.pos < self.len && (self.buf[self.pos] == b'E' || self.buf[self.pos] == b'e') {
            self.pos += 1;
            let sign = self.consume_sign();
            let mut offset = 0;
            if self.consume_int(&mut offset)? == 0 {
                return Err(Error::Unknown(vec![]))
            }
            if !sign {
                offset = -offset;
            }
            pow += offset;
        }
        if !sign {
            int = -int
        }

        Ok(if pow == 0 {
            int as f64
        } else  if pow < 0 {
            int as f64 / (10.0f64).powi(-pow as i32)
        } else {
            int as f64 * (10.0f64).powi(pow as i32)
        })
    }

    pub fn next<'a>(&'a mut self) -> Option<Result<Lexeme<'a>>> {
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
