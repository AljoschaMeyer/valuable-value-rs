use std::convert::TryInto;
use std::slice::SliceIndex;
use std::ops::{AddAssign, MulAssign, Neg};
use std::fmt;
use std::str::FromStr;

use strtod2::strtod;
use thiserror::Error;

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde::de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess,
    VariantAccess, Visitor,
};

use crate::parser_helper::{self, ParserHelper, is_hex_digit, is_digit, is_binary_digit, is_hex_digit_or_underscore, is_digit_or_underscore, is_binary_digit_or_underscore};

/// Everything that can go wrong during deserialization.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum DecodeError {
    /// Custom, stringly-typed error.
    #[error("{0}")]
    Message(String),
    /// A generic syntax error. Any valid vv would have been ok, but alas...
    #[error("invalid syntax")]
    Syntax,
    /// Encountered whitespace in canonic or compact mode.
    #[error("whitespace is disallowed")]
    Whitespace,
    /// Encountered compact encoding in human-readable mode.
    #[error("compact encoding is disallowed")]
    Compact,
    /// Encountered human-readable encoding in canonic or compact mode.
    #[error("human-readable encoding is disallowed")]
    HumanReadable,
    #[error("comments must be valid UTF-8")]
    CommentNotUtf8,
    #[error("an int must have at least one digit")]
    IntNoDigits,
    #[error("ints must be between -2^63 and 2^63 - 1 (inclusive)")]
    IntOutOfBounds,
    #[error("reached end of input while decoding a compact int of width {0}")]
    CompactIntShort(usize),
    #[error("canonicity requires that the integer is encoded with fewer bytes")]
    IntCanonicTooWide,
    #[error("reached end of input while decoding a compact float")]
    CompactFloatShort,
    #[error("a float must include a point")]
    FloatNoPoint,
    #[error("a float must have at least one digit preceding the point")]
    FloatNoLeadingDigits,
    #[error("a float must have at least one digit following the point")]
    FloatNoTrailingDigits,
    #[error("a float must have at least one digit following the exponent")]
    FloatNoExponentDigits,
    #[error("canonicity requires that NaN is encoded as eight 0xff bytes")]
    CanonicNaN,
    #[error("reached end of input while decoding a compact array of width {0}")]
    CompactArrayShort(usize),
    #[error("canonicity requires that the array count is encoded with fewer bytes")]
    ArrayCanonicTooWide,
    #[error("array count may not exceed 2^63 - 1")]
    ArrayTooLong,
    /// Expected a comma (`,`) to separate collection elements.
    #[error("array items must be separated by a comma")]
    Comma,
    // /// Expected a colon (`:`) to separate a key from a value.
    // Colon,
    // /// Expected a unicode escape (because we just parsed a unicode escape of a leading
    // /// surrogate codepoint).
    // UnicodeEscape,
    // // /// Could not merge two unicode escapes into a single code point.
    // // SurrogatePair(InvalidUtf16Tuple),
    // /// A unicode escape encoded a trailing surrogate codepoint without a preceding
    // /// leading surrogate codepoint.
    // TrailingSurrogate,
    // /// A string contained an unescaped control code point.
    // UnescapedControlCodePoint,
    // /// A string contained a backslash followed by a non-escape character.
    // InvalidEscape,
    // /// A string literal contains a non-utf8 byte sequence.
    // InvalidUtf8String,
    // /// A number is valid json but it evaluates to -0 or an infinity
    // InvalidNumber,
    // /// The input contained valid json followed by at least one non-whitespace byte.
    // TrailingCharacters,
    /// Attempted to parse a number as an `i8` that was out of bounds.
    #[error("i8 out of bounds")]
    OutOfBoundsI8,
    /// Attempted to parse a number as an `i16` that was out of bounds.
    #[error("i16 out of bounds")]
    OutOfBoundsI16,
    /// Attempted to parse a number as an `i32` that was out of bounds.
    #[error("i32 out of bounds")]
    OutOfBoundsI32,
    /// Attempted to parse a number as an `i64` that was less than -2^53 or greater than 2^53.
    #[error("i64 out of bounds")]
    OutOfBoundsI64,
    /// Attempted to parse a number as an `u8` that was out of bounds.
    #[error("u8 out of bounds")]
    OutOfBoundsU8,
    /// Attempted to parse a number as an `u16` that was out of bounds.
    #[error("u16 out of bounds")]
    OutOfBoundsU16,
    /// Attempted to parse a number as an `u32` that was out of bounds.
    #[error("u32 out of bounds")]
    OutOfBoundsU32,
    /// Attempted to parse a number as an `u64` that was greater than 2^53.
    #[error("u64 out of bounds")]
    OutOfBoundsU64,
    #[error("expected nil")]
    ExpectedNil,
    #[error("expected bool")]
    ExpectedBool,
    #[error("expected int")]
    ExpectedInt,
    #[error("expected float")]
    ExpectedFloat,
    #[error("expected array")]
    ExpectedArray,
    // /// Expected an object, found something else.
    // ExpectedObject,
    // /// Expected an enum, found something else.
    // ExpectedEnum,
}

impl de::Error for DecodeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        DecodeError::Message(msg.to_string())
    }
}

type Error = parser_helper::Error<DecodeError>;

/// A structure that deserializes valuable values.
///
/// https://github.com/AljoschaMeyer/valuable-value/blob/main/README.md
pub struct GenericSyntaxHelper<'de> {
    p: ParserHelper<'de>,
}

pub enum Number {
    F(f64),
    I(i64),
}

impl<'de> GenericSyntaxHelper<'de> {
    pub fn new(input: &'de [u8], enc: Encoding) -> Self {
        GenericSyntaxHelper {
            p: ParserHelper::new(input),
            enc,
        }
    }

    pub fn position(&self) -> usize {
        self.p.position()
    }

    pub fn spaces(&mut self) -> Result<(), Error> {
        match self.enc {
            Canonic | Compact => {
                match self.p.peek_or_end() {
                    Some(0x09) | Some(0x0a) | Some(0x0d) | Some(0x20) | Some(0x23) => {
                        return self.p.fail(DecodeError::Whitespace);
                    }
                    Some(_) | None => return Ok(()),
                }
            }

            HumanReadable | Hybrid => {
                loop {
                    match self.p.peek_or_end() {
                        Some(0x09) | Some(0x0a) | Some(0x0d) | Some(0x20) => self.p.advance(1),
                        Some(0x23) => self.comment()?,
                        Some(_) | None => return Ok(()),
                    }
                }
            }
        }
    }

    pub fn comment(&mut self) -> Result<(), Error> {
        let start = self.p.position();
        self.p.advance(1); // #
        loop {
            match self.p.next_or_end() {
                Some(0x0a) | None => {
                    match std::str::from_utf8(self.p.slice(start..self.p.position())) {
                        Ok(_) => return Ok(()),
                        Err(_) => return self.p.fail_at_position(DecodeError::CommentNotUtf8, start),
                    }
                }
                Some(_) => {}
            }
        }
    }

    pub fn peek_spaces(&mut self) -> Result<u8, Error> {
        self.spaces()?;
        self.p.peek()
    }

    pub fn parse_int(&mut self) -> Result<i64, Error> {
        let start = self.p.position();

        let negative = self.p.advance_over(b"-");
        let has_sign = negative || self.p.advance_over(b"+");

        let is_hex = !has_sign && self.p.advance_over(b"0x");
        let is_binary = !is_hex && (!has_sign && self.p.advance_over(b"0b"));

        if is_hex {
            if !is_hex_digit(self.p.peek()?) {
                return self.p.fail(DecodeError::IntNoDigits);
            }

            let start = self.p.position();
            self.p.skip(is_hex_digit_or_underscore);

            let digits_with_underscores = unsafe { std::str::from_utf8_unchecked(self.p.slice(start..self.p.position())) };
            let without_underscores = digits_with_underscores.replace("_", "");
            match i64::from_str_radix(&without_underscores, 16) {
                Ok(n) => return Ok(n),
                Err(_) => return self.p.fail(DecodeError::IntOutOfBounds),
            }
        } else if is_binary {
            if !is_binary_digit(self.p.peek()?) {
                return self.p.fail(DecodeError::IntNoDigits);
            }

            let start = self.p.position();
            self.p.skip(is_binary_digit_or_underscore);

            let digits_with_underscores = unsafe { std::str::from_utf8_unchecked(self.p.slice(start..self.p.position())) };
            let without_underscores = digits_with_underscores.replace("_", "");
            match i64::from_str_radix(&without_underscores, 2) {
                Ok(n) => return Ok(n),
                Err(_) => return self.p.fail(DecodeError::IntOutOfBounds),
            }
        } else {
            if !is_digit(self.p.peek()?) {
                if has_sign {
                    return self.p.fail(DecodeError::IntNoDigits);
                } else {
                    return self.p.fail(DecodeError::ExpectedInt);
                }
            }

            self.p.skip(is_digit_or_underscore);

            let digits_with_underscores = unsafe { std::str::from_utf8_unchecked(self.p.slice(start..self.p.position())) };
            let without_underscores = digits_with_underscores.replace("_", "");
            match i64::from_str_radix(&without_underscores, 10) {
                Ok(n) => return Ok(n),
                Err(_) => return self.p.fail(DecodeError::IntOutOfBounds),
            }
        }
    }

    pub fn parse_float(&mut self) -> Result<f64, Error> {
        let start = self.p.position();

        let negative = self.p.advance_over(b"-");
        let has_sign = negative || self.p.advance_over(b"+");

        match self.p.peek()? {
            0x49 => {
                self.p.expect_bytes(b"Inf", DecodeError::ExpectedFloat)?;
                return Ok(if negative { f64::NEG_INFINITY } else { f64::INFINITY });
            }
            0x4e => {
                self.p.expect_bytes(b"NaN", DecodeError::ExpectedFloat)?;
                return Ok(f64::NAN);
            }
            _ => {}
        }

        if !is_digit(self.p.peek()?) {
            if has_sign {
                return self.p.fail(DecodeError::FloatNoLeadingDigits);
            } else {
                return self.p.fail(DecodeError::ExpectedFloat);
            }
        }
        self.p.skip(is_digit_or_underscore);

        self.p.expect('.' as u8, DecodeError::FloatNoPoint)?;

        if !is_digit(self.p.peek()?) {
            return self.p.fail(DecodeError::FloatNoTrailingDigits);
        }
        self.p.skip(is_digit_or_underscore);

        if let 0x45 | 0x65 = self.p.peek()? {
            self.p.advance(1);
            let negative = self.p.advance_over(b"-");
            if !negative {
                self.p.advance_over(b"+");
            }

            if !is_digit(self.p.peek()?) {
                return self.p.fail(DecodeError::FloatNoExponentDigits);
            }
            self.p.skip(is_digit_or_underscore);
        }

        let digits_with_underscores = unsafe { std::str::from_utf8_unchecked(self.p.slice(start..self.p.position())) };
        let without_underscores = digits_with_underscores.replace("_", "");
        match f64::from_str(&without_underscores) {
            Ok(n) => return Ok(n),
            Err(_) => unreachable!("Prior parsing should have ensured a valid input to f64::from_str"),
        }
    }

    pub fn parse_number(&mut self) -> Result<Number, Error> {
        let start = self.p.position();

        let negative = self.p.advance_over(b"-");
        let has_sign = negative || self.p.advance_over(b"+");

        match self.p.peek()? {
            0x49 => {
                self.p.expect_bytes(b"Inf", DecodeError::ExpectedFloat)?;
                return Ok(if negative { Number::F(f64::NEG_INFINITY) } else { Number::F(f64::INFINITY) });
            }
            0x4e => {
                self.p.expect_bytes(b"NaN", DecodeError::ExpectedFloat)?;
                return Ok(Number::F(f64::NAN));
            }
            _ => {}
        }

        let is_hex = !has_sign && self.p.advance_over(b"0x");
        let is_binary = !is_hex && (!has_sign && self.p.advance_over(b"0b"));

        if is_hex {
            if !is_hex_digit(self.p.peek()?) {
                return self.p.fail(DecodeError::IntNoDigits);
            }

            let start = self.p.position();
            self.p.skip(is_hex_digit_or_underscore);

            let digits_with_underscores = unsafe { std::str::from_utf8_unchecked(self.p.slice(start..self.p.position())) };
            let without_underscores = digits_with_underscores.replace("_", "");
            match i64::from_str_radix(&without_underscores, 16) {
                Ok(n) => return Ok(Number::I(n)),
                Err(_) => return self.p.fail(DecodeError::IntOutOfBounds),
            }
        } else if is_binary {
            if !is_binary_digit(self.p.peek()?) {
                return self.p.fail(DecodeError::IntNoDigits);
            }

            let start = self.p.position();
            self.p.skip(is_binary_digit_or_underscore);

            let digits_with_underscores = unsafe { std::str::from_utf8_unchecked(self.p.slice(start..self.p.position())) };
            let without_underscores = digits_with_underscores.replace("_", "");
            match i64::from_str_radix(&without_underscores, 2) {
                Ok(n) => return Ok(Number::I(n)),
                Err(_) => return self.p.fail(DecodeError::IntOutOfBounds),
            }
        } else {
            if !is_digit(self.p.peek()?) {
                if has_sign {
                    return self.p.fail(DecodeError::IntNoDigits);
                } else {
                    return self.p.fail(DecodeError::ExpectedInt);
                }
            }

            self.p.skip(is_digit_or_underscore);

            match self.p.peek::<i8>() {
                Ok(0x2e) => {
                    self.p.advance(1);
                    if !is_digit(self.p.peek()?) {
                        return self.p.fail(DecodeError::FloatNoTrailingDigits);
                    }
                    self.p.skip(is_digit_or_underscore);

                    if let Ok(0x45 | 0x65) = self.p.peek::<i8>() {
                        self.p.advance(1);
                        let negative = self.p.advance_over(b"-");
                        if !negative {
                            self.p.advance_over(b"+");
                        }

                        if !is_digit(self.p.peek()?) {
                            return self.p.fail(DecodeError::FloatNoExponentDigits);
                        }
                        self.p.skip(is_digit_or_underscore);
                    }

                    let digits_with_underscores = unsafe { std::str::from_utf8_unchecked(self.p.slice(start..self.p.position())) };
                    let without_underscores = digits_with_underscores.replace("_", "");
                    match f64::from_str(&without_underscores) {
                        Ok(n) => return Ok(Number::F(n)),
                        Err(_) => unreachable!("Prior parsing should have ensured a valid input to f64::from_str"),
                    }
                }

                _ => {
                    let digits_with_underscores = unsafe { std::str::from_utf8_unchecked(self.p.slice(start..self.p.position())) };
                    let without_underscores = digits_with_underscores.replace("_", "");
                    match i64::from_str_radix(&without_underscores, 10) {
                        Ok(n) => return Ok(Number::I(n)),
                        Err(_) => return self.p.fail(DecodeError::IntOutOfBounds),
                    }
                }
            }
        }
    }
}
