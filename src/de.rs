use std::convert::TryInto;
use std::slice::SliceIndex;
use std::ops::{AddAssign, MulAssign, Neg};
use std::fmt;

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
    // /// Expected a comma (`,`) to separate collection elements.
    // Comma,
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
    // /// Expected an array, found something else.
    // ExpectedArray,
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

#[derive(PartialEq, Eq, Debug)]
pub enum Encoding {
    Canonic,
    Compact,
    HumanReadable,
    Hybrid,
}
use Encoding::*;

/// A structure that deserializes valuable values.
///
/// https://github.com/AljoschaMeyer/valuable-value/blob/main/README.md
pub struct VVDeserializer<'de> {
    p: ParserHelper<'de>,
    enc: Encoding,
}

enum Number {
    F(f64),
    I(i64),
}

impl<'de> VVDeserializer<'de> {
    pub fn new(input: &'de [u8], enc: Encoding) -> Self {
        VVDeserializer {
            p: ParserHelper::new(input),
            enc,
        }
    }

    pub fn position(&self) -> usize {
        self.p.position()
    }

    // Errors if human-readable input is currently forbidden. Error position is n byte before the current position.
    fn human(&self, n: usize) -> Result<(), Error> {
        match self.enc {
            HumanReadable | Hybrid => Ok(()),
            Canonic | Compact => self.p.fail_at_position(DecodeError::HumanReadable, self.p.position() - n),
        }
    }

    // Errors if compact input is currently forbidden. Error position is n byte before the current position.
    fn compact(&self, n: usize) -> Result<(), Error> {
        match self.enc {
            Canonic | Compact | Hybrid => Ok(()),
            HumanReadable => self.p.fail_at_position(DecodeError::Compact, self.p.position() - n),
        }
    }

    fn spaces(&mut self) -> Result<(), Error> {
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

    fn comment(&mut self) -> Result<(), Error> {
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

    fn peek_spaces(&mut self) -> Result<u8, Error> {
        self.spaces()?;
        self.p.peek()
    }

    fn parse_nil_compact(&mut self) -> Result<(), Error> {
        self.compact(0)?;
        self.p.expect(0b1_010_1100, DecodeError::ExpectedNil)
    }

    fn parse_nil_human(&mut self) -> Result<(), Error> {
        self.human(0)?;
        self.p.expect_bytes(b"nil", DecodeError::ExpectedNil)
    }

    fn parse_nil(&mut self) -> Result<(), Error> {
        match self.p.peek()? {
            0b1_010_1100 => self.parse_nil_compact(),
            0x6e => self.parse_nil_human(),
            _ => self.p.fail(DecodeError::ExpectedNil)?,
        }
    }

    fn parse_bool_compact(&mut self) -> Result<bool, Error> {
        self.compact(0)?;
        match self.p.next()? {
            0b1_010_1101 => Ok(false),
            0b1_010_1110 => Ok(true),
            _ => self.p.fail_at_position(DecodeError::ExpectedBool, self.p.position() - 1),
        }
    }

    fn parse_bool_human(&mut self) -> Result<bool, Error> {
        self.human(0)?;
        match self.p.next()? {
            0x66 => {
                self.p.expect_bytes(b"alse", DecodeError::ExpectedBool)?;
                return Ok(false);
            }
            0x74 => {
                self.p.expect_bytes(b"rue", DecodeError::ExpectedBool)?;
                return Ok(true);
            }
            _ => self.p.fail_at_position(DecodeError::ExpectedBool, self.p.position() - 1),
        }
    }

    fn parse_bool(&mut self) -> Result<bool, Error> {
        match self.p.peek()? {
            0b1_010_1101 | 0b1_010_1110 => self.parse_bool_compact(),
            0x66 | 0x74 => self.parse_bool_human(),
            _ => self.p.fail(DecodeError::ExpectedBool)?,
        }
    }

    fn parse_int_compact(&mut self) -> Result<i64, Error> {
        self.compact(0)?;
        match self.p.next()? {
            b if b & 0b1_111_0000 == 0b1_011_0000 => {
                if b == 0b1_011_1111 {
                    let start = self.p.position();
                    self.p.advance_or(8, DecodeError::CompactIntShort(8))?;
                    let n = i64::from_be_bytes(self.p.slice(start..start + 8).try_into().unwrap());
                    if self.enc == Encoding::Canonic && (i32::MIN as i64) <= n && n <= (i32::MAX as i64) {
                        return self.p.fail_at_position(DecodeError::IntCanonicTooWide, start);
                    }
                    return Ok(n);
                } else if b == 0b1_011_1110 {
                    let start = self.p.position();
                    self.p.advance_or(4, DecodeError::CompactIntShort(4))?;
                    let n = i32::from_be_bytes(self.p.slice(start..start + 4).try_into().unwrap()) as i64;
                    if self.enc == Encoding::Canonic && (i16::MIN as i64) <= n && n <= (i16::MAX as i64) {
                        return self.p.fail_at_position(DecodeError::IntCanonicTooWide, start);
                    }
                    return Ok(n);
                } else if b == 0b1_011_1101 {
                    let start = self.p.position();
                    self.p.advance_or(2, DecodeError::CompactIntShort(2))?;
                    let n = i16::from_be_bytes(self.p.slice(start..start + 2).try_into().unwrap()) as i64;
                    if self.enc == Encoding::Canonic && (i8::MIN as i64) <= n && n <= (i8::MAX as i64) {
                        return self.p.fail_at_position(DecodeError::IntCanonicTooWide, start);
                    }
                    return Ok(n);
                } else if b == 0b1_011_1100 {
                    let start = self.p.position();
                    self.p.advance_or(1, DecodeError::CompactIntShort(1))?;
                    let n = i8::from_be_bytes(self.p.slice(start..start + 1).try_into().unwrap()) as i64;
                    if self.enc == Encoding::Canonic && 0 <= n && n <= 11 {
                        return self.p.fail_at_position(DecodeError::IntCanonicTooWide, start);
                    }
                    return Ok(n);
                } else {
                    return Ok((u8::from_be_bytes([b & 0b0_000_1111])) as i64);
                }
            }
            _ => self.p.fail_at_position(DecodeError::ExpectedInt, self.p.position() - 1),
        }
    }

    fn parse_int_human(&mut self) -> Result<i64, Error> {
        self.human(0)?;
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

    fn parse_int(&mut self) -> Result<i64, Error> {
        match self.p.peek()? {
            b if b & 0b1_111_0000 == 0b1_011_0000 => self.parse_int_compact(),
            b if b == ('+' as u8) || b == ('-' as u8) || is_digit(b) => self.parse_int_human(),
            _ => self.p.fail(DecodeError::ExpectedInt)?,
        }
    }

    fn parse_number(&mut self) -> Result<Number, Error> {
        match self.p.peek()? {
            b if b == ('+' as u8) || b == ('-' as u8) || is_digit(b) => {
                Ok(Number::I(self.parse_int_human()?)) // TODO floats
            }
            _ => unreachable!(),
        }
    }


    // // Parses the four characters of a unicode escape sequence and returns the codepoint they
    // // encode. Json only allows escaping codepoints in the BMP, that's why it fits into a `u16`.
    // fn parse_unicode_escape(&mut self) -> Result<u16, DecodeJsonError> {
    //     let start = self.position();
    //
    //     for _ in 0..4 {
    //         self.expect_pred(is_hex_digit, ErrorCode::HexDigit)?;
    //     }
    //
    //     u16::from_str_radix(
    //         unsafe { std::str::from_utf8_unchecked(&self.slice(start..start + 4)) },
    //         16,
    //     )
    //     .map_err(|_| unreachable!("We already checked for valid input"))
    // }

    // fn foo(&mut self) -> Result<Number, DecodeJsonError> {
    //     let start = self.position;
    //     let has_sign = if self.starts_with("+") || self.starts_with("-") {
    //         self.advance(1);
    //         true
    //     } else {
    //         false
    //     };
    //     let is_hex = if self.starts_with("0x") {
    //         self.advance(2);
    //         true
    //     } else {
    //         false
    //     };
    //
    //     if is_hex {
    //         if !self.peek()?.is_ascii_hexdigit() {
    //             return self.err(ParseError::HexIntNoDigits);
    //         }
    //         self.skip(|c: char| c.is_ascii_hexdigit());
    //
    //         let end = start.len() - self.position.len();
    //         self.ws();
    //         let raw = if has_sign {
    //             let mut buf = start[..1].to_string();
    //             buf.push_str(&start[3..end]);
    //             buf
    //         } else {
    //             start[2..end].to_string()
    //         };
    //
    //         match i64::from_str_radix(&raw, 16) {
    //             Ok(n) => return Ok(Expression::Int(n)),
    //             Err(_) => return self.err(ParseError::HexIntOutOfBounds),
    //         }
    //     } else {
    //         if !self.peek()?.is_ascii_digit() {
    //             return self.err(ParseError::DecIntNoDigits);
    //         }
    //         self.skip(|c: char| c.is_ascii_digit());
    //
    //         let is_float = match self.peek_or_end() {
    //             Some('.') => {
    //                 self.advance(1);
    //                 true
    //             }
    //             _ => false,
    //         };
    //
    //         if is_float {
    //             // if is_float {
    //             //     let (i, _) = try_parse!(i, take_while1!(|c: char| c.is_ascii_digit()));
    //             //     let (i, _) = try_parse!(i, opt!(do_parse!(
    //             //         one_of!("eE") >>
    //             //         opt!(one_of!("+-")) >>
    //             //         take_while1!(|c: char| c.is_ascii_digit()) >>
    //             //         (())
    //             //     )));
    //             //     let end = i;
    //             //
    //             //     let raw = &start[..start.len() - end.len()];
    //             //     let f = strtod(raw).unwrap();
    //             //     if f.is_finite() {
    //             //         return Ok((i, Value::float(f)));
    //             //     } else {
    //             //         return Err(Err::Failure(Context::Code(i, ErrorKind::Custom(2))));
    //             //     }
    //             // }
    //             self.ws();
    //             unimplemented!();
    //         } else {
    //             let end = start.len() - self.position.len();
    //             self.ws();
    //             match i64::from_str_radix(&start[..end], 10) {
    //                 Ok(n) => return Ok(Expression::Int(n)),
    //                 Err(_) => return self.err(ParseError::DecIntOutOfBounds),
    //             }
    //         }
    //     }
    // }

    // fn parse_number_except(
    //     &mut self,
    //     pred: fn(f64) -> bool,
    //     err: ErrorCode,
    // ) -> Result<f64, DecodeJsonError> {
    //     let pos = self.position();
    //     let f = self.parse_number()?;
    //     if pred(f) {
    //         Ok(f)
    //     } else {
    //         self.fail_at_position(err, pos)
    //     }
    // }

    // fn parse_number(&mut self) -> Result<f64, DecodeJsonError> {
    //     let start = self.position();
    //
    //     // trailing `-`
    //     match self.peek() {
    //         Ok(0x2D) => self.advance(1),
    //         Ok(_) => {}
    //         Err(_) => return self.fail(ErrorCode::ExpectedNumber),
    //     }
    //
    //     let next = self.next()?;
    //     match next {
    //         // first digit `0` must be followed by `.`
    //         0x30 => {}
    //         // first digit nonzero, may be followed by more digits until the `.`
    //         0x31..=0x39 => self.skip(is_digit),
    //         _ => return self.fail_at_position(ErrorCode::ExpectedNumber, start),
    //     }
    //
    //     // `.`, followed by many1 digits
    //     if let Some(0x2E) = self.peek_or_end() {
    //         self.advance(1);
    //         self.expect_pred(is_digit, ErrorCode::Digit)?;
    //         self.skip(is_digit);
    //     }
    //
    //     // `e` or `E`, followed by an optional sign and many1 digits
    //     match self.peek_or_end() {
    //         Some(0x45) | Some(0x65) => {
    //             self.advance(1);
    //
    //             // optional `+` or `-`
    //             if self.peek()? == 0x2B || self.peek()? == 0x2D {
    //                 self.advance(1);
    //             }
    //
    //             // many1 digits
    //             self.expect_pred(is_digit, ErrorCode::Digit)?;
    //             self.skip(is_digit);
    //         }
    //         _ => {}
    //     }
    //
    //     // done parsing the number, convert it to a rust value
    //     let f =
    //         strtod(unsafe { std::str::from_utf8_unchecked(self.slice(start..self.position())) })
    //             .unwrap(); // We already checked that the input is a valid number
    //
    //     Ok(f)
    // }
    //
    // // Return a slice beginning and ending with 0x22 (`"`)
    // fn parse_naive_string(&mut self) -> Result<&'de [u8], DecodeJsonError> {
    //     self.expect(0x22, ErrorCode::ExpectedString)?;
    //     let start = self.position();
    //
    //     while self.next()? != 0x22 {
    //         // noop
    //     }
    //
    //     Ok(self.slice(start..self.position()))
    // }

    // fn parse_string(&mut self) -> Result<String, DecodeJsonError> {
    //     self.expect(0x22, ErrorCode::ExpectedString)?;
    //
    //     let mut decoded = String::new();
    //
    //     loop {
    //         match self.peek()? {
    //             // terminating `"`, return the decoded string
    //             0x22 => {
    //                 self.advance(1);
    //                 return Ok(decoded);
    //             }
    //
    //             // `\` introduces an escape sequence
    //             0x5C => {
    //                 let pos = self.position();
    //                 self.advance(1);
    //
    //                 match self.next()? {
    //                     // single character escape sequences
    //                     0x22 => decoded.push('\u{22}'), // `\"`
    //                     0x5C => decoded.push('\u{5C}'), // `\\`
    //                     0x2F => decoded.push('\u{2F}'), // `\/`
    //                     0x62 => decoded.push('\u{08}'), // `\b`
    //                     0x66 => decoded.push('\u{0C}'), // `\f`
    //                     0x6E => decoded.push('\u{0A}'), // `\n`
    //                     0x72 => decoded.push('\u{0D}'), // `\r`
    //                     0x74 => decoded.push('\u{09}'), // `\t`
    //
    //                     // unicode escape sequences
    //                     0x75 => {
    //                         let cp = self.parse_unicode_escape()?;
    //
    //                         match code_unit_type(cp) {
    //                             CodeUnitType::Valid => decoded
    //                                 .push(unsafe { std::char::from_u32_unchecked(cp as u32) }),
    //
    //                             CodeUnitType::LeadingSurrogate => {
    //                                 // the unicode escape was for a leading surrogate, which
    //                                 // must be followed by another unicode escape which is a
    //                                 // trailing surrogate
    //                                 self.expect(0x5C, ErrorCode::UnicodeEscape)?;
    //                                 self.expect(0x75, ErrorCode::UnicodeEscape)?;
    //                                 let cp2 = self.parse_unicode_escape()?;
    //
    //                                 match Utf16Char::from_tuple((cp, Some(cp2))) {
    //                                     Ok(c) => decoded.push(c.into()),
    //                                     Err(e) => {
    //                                         return self
    //                                             .fail_at_position(ErrorCode::SurrogatePair(e), pos)
    //                                     }
    //                                 }
    //                             }
    //
    //                             CodeUnitType::TrailingSurrogate => {
    //                                 return self.fail_at_position(ErrorCode::TrailingSurrogate, pos)
    //                             }
    //                         }
    //                     }
    //
    //                     // Nothing else may follow an unescaped `\`
    //                     _ => return self.fail_at_position(ErrorCode::InvalidEscape, pos),
    //                 }
    //             }
    //
    //             // the control code points must be escaped
    //             0x00..=0x1F => return self.fail(ErrorCode::UnescapedControlCodePoint),
    //
    //             // a regular utf8-encoded code point (unless it is malformed)
    //             _ => match Utf8Char::from_slice_start(self.rest()) {
    //                 Err(_) => return self.fail(ErrorCode::InvalidUtf8String),
    //                 Ok((_, len)) => unsafe {
    //                     decoded.push_str(std::str::from_utf8_unchecked(&self.rest()[..len]));
    //                     self.advance(len);
    //                 },
    //             },
    //         }
    //     }
    // }
}

impl<'a, 'de> de::Deserializer<'de> for &'a mut VVDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.peek_spaces()? {
            0b1_010_1100 => {
                self.parse_nil_compact()?;
                visitor.visit_unit()
            }
            0x6e => {
                self.parse_nil_human()?;
                visitor.visit_unit()
            }

            0b1_010_1101 | 0b1_010_1110 => visitor.visit_bool(self.parse_bool_compact()?),
            0x66 | 0x74 => visitor.visit_bool(self.parse_bool_human()?),

            b if b & 0b1_111_0000 == 0b1_011_0000 => visitor.visit_i64(self.parse_int_compact()?),
            b if b == ('+' as u8) || b == ('-' as u8) || is_digit(b) => {
                match self.parse_number()? {
                    Number::I(n) => visitor.visit_i64(n),
                    Number::F(n) => visitor.visit_f64(n),
                }
            }
            _ => self.p.fail(DecodeError::Syntax),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.spaces()?;
        visitor.visit_bool(self.parse_bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.spaces()?;
        let start = self.p.position();
        let n = self.parse_int()?;
        if n < std::i8::MIN as i64 || n > std::i8::MAX as i64 {
            return self.p.fail_at_position(DecodeError::OutOfBoundsI8, start);
        } else {
            visitor.visit_i8(n as i8)
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.spaces()?;
        let start = self.p.position();
        let n = self.parse_int()?;
        if n < std::i16::MIN as i64 || n > std::i16::MAX as i64 {
            return self.p.fail_at_position(DecodeError::OutOfBoundsI16, start);
        } else {
            visitor.visit_i16(n as i16)
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.spaces()?;
        let start = self.p.position();
        let n = self.parse_int()?;
        if n < std::i32::MIN as i64 || n > std::i32::MAX as i64 {
            return self.p.fail_at_position(DecodeError::OutOfBoundsI32, start);
        } else {
            visitor.visit_i32(n as i32)
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_int()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.spaces()?;
        let start = self.p.position();
        let n = self.parse_int()?;
        if n < 0 || n > std::u8::MAX as i64 {
            return self.p.fail_at_position(DecodeError::OutOfBoundsU8, start);
        } else {
            visitor.visit_u8(n as u8)
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.spaces()?;
        let start = self.p.position();
        let n = self.parse_int()?;
        if n < 0 || n > std::u16::MAX as i64 {
            return self.p.fail_at_position(DecodeError::OutOfBoundsU16, start);
        } else {
            visitor.visit_u16(n as u16)
        }
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.spaces()?;
        let start = self.p.position();
        let n = self.parse_int()?;
        if n < 0 || n > std::u32::MAX as i64 {
            return self.p.fail_at_position(DecodeError::OutOfBoundsU32, start);
        } else {
            visitor.visit_u32(n as u32)
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.spaces()?;
        let start = self.p.position();
        let n = self.parse_int()?;
        if n < 0 {
            return self.p.fail_at_position(DecodeError::OutOfBoundsU64, start);
        } else {
            visitor.visit_u64(n as u64)
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // visitor.visit_f32(self.parse_number()? as f32)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // visitor.visit_f64(self.parse_number()?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // let pos = self.position();
        // let s = self.parse_string()?;
        // let mut chars = s.chars();
        //
        // match chars.next() {
        //     None => self.fail_at_position(ErrorCode::NotAChar, pos),
        //     Some(c) => match chars.next() {
        //         None => visitor.visit_char(c),
        //         Some(_) => self.fail_at_position(ErrorCode::NotAChar, pos),
        //     },
        // }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // // We can't reference json strings directly since they contain escape sequences.
        // // For the conversion, we need to allocate an owned buffer, so always do owned
        // // deserialization.
        // self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // visitor.visit_string(self.parse_string()?)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // // We can't reference bytes directly since they are stored as base64 strings.
        // // For the conversion, we need to allocate an owned buffer, so always do owned
        // // deserialization.
        // self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // let pos = self.position();
        // match base64::decode(self.parse_naive_string()?) {
        //     Ok(buf) => visitor.visit_byte_buf(buf),
        //     Err(e) => self.fail_at_position(ErrorCode::Base64(e), pos),
        // }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // if self.rest().starts_with(b"null") {
        //     self.advance(4);
        //     visitor.visit_none()
        // } else {
        //     visitor.visit_some(self)
        // }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.spaces()?;
        self.parse_nil()?;
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // self.expect(0x5B, ErrorCode::ExpectedArray)?;
        // let value = visitor.visit_seq(CollectionAccessor::new(&mut self))?;
        // self.expect_ws(0x5D, ErrorCode::Syntax)?; // Can't fail
        // Ok(value)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // self.expect(0x7B, ErrorCode::ExpectedObject)?;
        // let value = visitor.visit_map(CollectionAccessor::new(&mut self))?;
        // self.expect_ws(0x7D, ErrorCode::Syntax)?; // Can't fail
        // Ok(value)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // let pos = self.position();
        // if self.peek()? == 0x22 {
        //     // Visit a unit variant.
        //     visitor.visit_enum(self.parse_string()?.into_deserializer())
        // } else if self.next()? == 0x7B {
        //     // Visit a newtype variant, tuple variant, or struct variant.
        //     let value = visitor.visit_enum(Enum::new(self))?;
        //     self.expect_ws(0x7D, ErrorCode::Syntax)?; // Can't fail
        //     Ok(value)
        // } else {
        //     self.fail_at_position(ErrorCode::ExpectedEnum, pos)
        // }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // self.deserialize_any(visitor)
    }
}

struct CollectionAccessor<'a, 'de> {
    des: &'a mut VVDeserializer<'de>,
    first: bool,
}

impl<'a, 'de> CollectionAccessor<'a, 'de> {
    fn new(des: &'a mut VVDeserializer<'de>) -> CollectionAccessor<'a, 'de> {
        CollectionAccessor { des, first: true }
    }
}

impl<'a, 'de> SeqAccess<'de> for CollectionAccessor<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        unimplemented!();
        // // Array ends at `]`
        // if let 0x5D = self.des.peek_ws()? {
        //     return Ok(None);
        // }
        //
        // // expect `,` before every item except the first
        // if self.first {
        //     self.first = false;
        // } else {
        //     self.des.expect_ws(0x2C, ErrorCode::Comma)?;
        // }
        //
        // self.des.peek_ws()?;
        //
        // seed.deserialize(&mut *self.des).map(Some)
    }
}

impl<'a, 'de> MapAccess<'de> for CollectionAccessor<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        unimplemented!();
        // // Object ends at `}`
        // if let 0x7D = self.des.peek_ws()? {
        //     return Ok(None);
        // }
        //
        // // expect `,` before every item except the first
        // if self.first {
        //     self.first = false;
        // } else {
        //     self.des.expect_ws(0x2C, ErrorCode::Comma)?;
        // }
        //
        // self.des.peek_ws()?;
        // seed.deserialize(&mut *self.des).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        unimplemented!();
        // self.des.expect_ws(0x3A, ErrorCode::Colon)?; // `:`
        //
        // self.des.peek_ws()?;
        // seed.deserialize(&mut *self.des)
    }
}

struct Enum<'a, 'de> {
    des: &'a mut VVDeserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(des: &'a mut VVDeserializer<'de>) -> Self {
        Enum { des }
    }
}

impl<'a, 'de> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        unimplemented!();
        // self.des.peek_ws()?;
        // let val = seed.deserialize(&mut *self.des)?;
        // self.des.expect_ws(0x3A, ErrorCode::Colon)?; // `:`
        //
        // self.des.peek_ws()?;
        // Ok((val, self))
    }
}

impl<'a, 'de> VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        unimplemented!();
        // eprintln!("wtf is this");
        // self.des.fail(ErrorCode::ExpectedString)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        unimplemented!();
        // seed.deserialize(self.des)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // de::Deserializer::deserialize_seq(self.des, visitor)
    }

    // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }` so
    // deserialize the inner map here.
    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // de::Deserializer::deserialize_map(self.des, visitor)
    }
}
