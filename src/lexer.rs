use std::io;


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

pub enum Lexeme {
    Lexeme(Vec<u8>),
    Unterminated,
    IOError(io::Error),
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
}

impl<T: io::Read> Iterator for Lexer<T> {
    type Item = Lexeme;

    fn next(&mut self) -> Option<Self::Item> {
        while match self.ensure_buffer() {
            Err(e) => return Some(Lexeme::IOError(e)),
            Ok(Buffer::Empty) => return None,
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
                match self.ensure_buffer() {
                    Err(e) => return Some(Lexeme::IOError(e)),
                    Ok(Buffer::Empty) => return Some(Lexeme::Unterminated),
                    Ok(Buffer::Within) => { self.pos += 1; break }
                    Ok(Buffer::Reset) => (), // continue
                }
            }
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
                match self.ensure_buffer() {
                    Err(e) => return Some(Lexeme::IOError(e)),
                    Ok(Buffer::Reset) => (), // continue
                    _ => break,
                }
            }
        }
        Some(Lexeme::Lexeme(result))
    }
}
