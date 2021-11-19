use std::fmt;
use std::slice::SliceIndex;

use thiserror::Error;

pub struct ParserHelper<'a> {
    input: &'a [u8],
    position: usize,
}

#[derive(Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[error("parse error at position {position}: {reason}")]
pub struct Error<E> {
    pub position: usize,
    pub reason: Reason<E>,
}

impl<E> Error<E> {
    pub fn new(position: usize, reason: E) -> Self {
        Error {
            position,
            reason: Reason::Other(reason),
        }
    }

    pub fn unexpected_end_of_input(position: usize) -> Self {
        Error {
            position,
            reason: Reason::UnexpectedEndOfInput,
        }
    }
}

impl<E: serde::de::Error> serde::de::Error for Error<E> {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::new(0, E::custom(msg))
    }
}

#[derive(Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Reason<E> {
    #[error("reached end of input")]
    UnexpectedEndOfInput,
    #[error("{0}")]
    Other(E),
}

impl<'a> ParserHelper<'a> {
    /// Parses from a slice of bytes.
    pub fn new(input: &'a [u8]) -> Self {
        ParserHelper {
            input,
            position: 0,
        }
    }

    /// Return the total length of the input.
    pub fn len(&self) -> usize {
        self.input.len()
    }

    /// Obtain a slice into the original input.
    pub fn slice<I: SliceIndex<[u8]>>(&self, i: I) -> &'a I::Output {
        &self.input[i]
    }

    /// Reference to portion of buffer yet to be parsed
    pub fn rest(&self) -> &'a [u8] {
        self.slice(self.position()..)
    }

    /// Current byte offset of buffer being parsed
    pub fn position(&self) -> usize {
        self.position
    }

    pub fn fail<T, E>(&self, reason: E) -> Result<T, Error<E>> {
        self.fail_at_position(reason, self.position())
    }

    pub fn fail_at_position<T, E>(&self, reason: E, position: usize) -> Result<T, Error<E>> {
        Err(Error::new(position, reason))
    }

    pub fn unexpected_end_of_input<T, E>(&self) -> Result<T, Error<E>> {
        Err(Error::unexpected_end_of_input(self.position()))
    }

    // Advance the input slice by some number of bytes.
    pub fn advance(&mut self, offset: usize) {
        self.position += offset;
    }

    /// Advance the input but only if it matches the given bytes, returns whether it did advance.
    pub fn advance_over(&mut self, expected: &[u8]) -> bool {
        if self.rest().starts_with(expected) {
            self.advance(expected.len());
            return true;
        } else {
            return false;
        }
    }

    // Advance the input slice by some number of bytes, returning the given error if not enough input is available.
    pub fn advance_or<E>(&mut self, offset: usize, e: E) -> Result<(), Error<E>> {
        let start = self.position;
        self.position += offset;
        if self.len() < self.position {
            return self.fail_at_position(e, start);
        } else {
            return Ok(());
        }
    }

    // Consumes the next byte and returns it.
    pub fn next<E>(&mut self) -> Result<u8, Error<E>> {
        if let Some(c) = self.input.get(self.position()) {
            self.advance(1);
            Ok(*c)
        } else {
            self.unexpected_end_of_input()
        }
    }

    // Consumes the next byte and returns it, or signals end of input as `None`.
    pub fn next_or_end(&mut self) -> Option<u8> {
        if let Some(c) = self.input.get(self.position()) {
            self.advance(1);
            Some(*c)
        } else {
            None
        }
    }

    // Consumes the expected byte, gives the given error if it is something else.
    pub fn expect<E>(&mut self, expected: u8, err: E) -> Result<(), Error<E>> {
        let pos = self.position();
        if self.next()? == expected {
            Ok(())
        } else {
            self.fail_at_position(err, pos)
        }
    }

    // Same as expect, but using a predicate.
    pub fn expect_pred<E>(&mut self, pred: fn(u8) -> bool, err: E) -> Result<(), Error<E>> {
        let pos = self.position();
        if pred(self.next()?) {
            Ok(())
        } else {
            self.fail_at_position(err, pos)
        }
    }

    // Returns the next byte without consuming it.
    pub fn peek<E>(&self) -> Result<u8, Error<E>> {
        if let Some(c) = self.input.get(self.position()) {
            Ok(*c)
        } else {
            self.unexpected_end_of_input()
        }
    }

    // Returns the next byte without consuming it, or signals end of input as `None`.
    pub fn peek_or_end(&self) -> Option<u8> {
        self.input.get(self.position()).copied()
    }

    // Skips values while the predicate returns true.
    pub fn skip(&mut self, pred: fn(u8) -> bool) {
        loop {
            match self.peek_or_end() {
                None => return,
                Some(peeked) => {
                    if pred(peeked) {
                        self.advance(1);
                    } else {
                        return;
                    }
                }
            }
        }
    }

    pub fn skip_ws(&mut self) {
        self.skip(is_ws)
    }

    // Consumes as much whitespace as possible, then peeks at the next non-whitespace byte.
    pub fn peek_ws<E>(&mut self) -> Result<u8, Error<E>> {
        self.skip_ws();
        self.peek()
    }

    pub fn expect_ws<E>(&mut self, exp: u8, err: E) -> Result<(), Error<E>> {
        self.skip_ws();
        self.expect(exp, err)
    }

    pub fn expect_bytes<E>(&mut self, exp: &[u8], err: E) -> Result<(), Error<E>> {
        if self.rest().starts_with(exp) {
            self.advance(exp.len());
            Ok(())
        } else {
            self.fail(err)
        }
    }
}

/// space (0x20), tab, newline, or carriage return
pub fn is_ws(byte: u8) -> bool {
    byte == 0x09 || byte == 0x0A || byte == 0x0D || byte == 0x20
}

pub fn is_digit(byte: u8) -> bool {
    byte.is_ascii_digit()
}

pub fn is_hex_digit(byte: u8) -> bool {
    byte.is_ascii_hexdigit()
}

pub fn is_binary_digit(byte: u8) -> bool {
    byte == ('0' as u8) || byte == ('1' as u8)
}

pub fn is_digit_or_underscore(byte: u8) -> bool {
    byte == ('_' as u8) || byte.is_ascii_digit()
}

pub fn is_hex_digit_or_underscore(byte: u8) -> bool {
    byte == ('_' as u8) || is_hex_digit(byte)
}

pub fn is_binary_digit_or_underscore(byte: u8) -> bool {
    byte == ('_' as u8) || is_binary_digit(byte)
}
