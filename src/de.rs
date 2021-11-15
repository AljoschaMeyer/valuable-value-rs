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

use crate::parser_helper::{self, ParserHelper};

// use crate::error::{Error, Result};

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
    #[error("expected a comment")]
    NoComment,
    #[error("comments must be valid UTF-8")]
    CommentNotUtf8,
    // /// Expected a comma (`,`) to separate collection elements.
    // Comma,
    // /// Expected a colon (`:`) to separate a key from a value.
    // Colon,
    // /// Expected a decimal digit. Didn't get one. Sad times.
    // Digit,
    // /// Expected hexadecimal digit as part of a unicode escape sequence in a string.
    // HexDigit,
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
    // /// Attempted to parse a number as an `i8` that was out of bounds.
    // OutOfBoundsI8,
    // /// Attempted to parse a number as an `i16` that was out of bounds.
    // OutOfBoundsI16,
    // /// Attempted to parse a number as an `i32` that was out of bounds.
    // OutOfBoundsI32,
    // /// Attempted to parse a number as an `i64` that was less than -2^53 or greater than 2^53.
    // OutOfBoundsI64,
    // /// Attempted to parse a number as an `u8` that was out of bounds.
    // OutOfBoundsU8,
    // /// Attempted to parse a number as an `u16` that was out of bounds.
    // OutOfBoundsU16,
    // /// Attempted to parse a number as an `u32` that was out of bounds.
    // OutOfBoundsU32,
    // /// Attempted to parse a number as an `u64` that was greater than 2^53.
    // OutOfBoundsU64,
    // // /// Chars are represented as strings that contain one unicode scalar value.
    // // NotAChar,
    // /// Expected a boolean, found something else.
    // ExpectedBool,
    // /// Expected a number, found something else.
    // ExpectedNumber,
    // /// Expected a string, found something else.
    // ExpectedString,
    /// Expected nil, found something else.
    #[error("expected nil")]
    ExpectedNil,
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
        self.p.expect_ws('#' as u8, DecodeError::NoComment)?;
        loop {
            match self.p.next_or_end() {
                Some(0x0a) | None => {
                    match std::str::from_utf8(self.p.slice(start..)) {
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

    fn parse_nil(&mut self) -> Result<(), Error> {
        match self.p.next()? {
            0b1_010_1100 => {
                match self.enc {
                    Canonic | Compact | Hybrid => return Ok(()),
                    HumanReadable => return self.p.fail_at_position(DecodeError::Compact, self.p.position() - 1),
                }
            }
            0x6e => {
                match self.enc {
                    HumanReadable | Hybrid => {
                        return self.p.expect_bytes(b"il", DecodeError::ExpectedNil);
                    }
                    Canonic | Compact => return self.p.fail_at_position(DecodeError::HumanReadable, self.p.position() - 1),
                }
            }
            _ => return self.p.fail_at_position(DecodeError::ExpectedNil, self.p.position() - 1),
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

    // fn parse_bool(&mut self) -> Result<bool, DecodeJsonError> {
    //     match self.expect_bytes(b"true", ErrorCode::ExpectedBool) {
    //         Ok(()) => Ok(true),
    //         Err(_) => self
    //             .expect_bytes(b"false", ErrorCode::ExpectedBool)
    //             .map(|_| false),
    //     }
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
            0b1_010_1100 | 0x6e => {
                self.parse_nil()?;
                visitor.visit_unit()
            }
            _ => self.p.fail(DecodeError::Syntax),
        }
        // match self.peek_ws()? {
        //     0x66 => {
        //         if self.rest()[1..].starts_with(b"alse") {
        //             self.advance(5);
        //             visitor.visit_bool(false)
        //         } else {
        //             self.fail(ErrorCode::Syntax)
        //         }
        //     }
        //     0x74 => {
        //         if self.rest()[1..].starts_with(b"rue") {
        //             self.advance(4);
        //             visitor.visit_bool(true)
        //         } else {
        //             self.fail(ErrorCode::Syntax)
        //         }
        //     }
        //     0x22 => self.deserialize_str(visitor),
        //     0x5B => self.deserialize_seq(visitor),
        //     0x7B => self.deserialize_map(visitor),
        //     0x2D | 0x30..=0x39 => self.deserialize_f64(visitor),
        //     _ => self.fail(ErrorCode::Syntax),
        // }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // visitor.visit_bool(self.parse_bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // let f = self.parse_int_except(
        //     |n| n < std::i8::MIN as i64 || n > std::i8::MAX as i64,
        //     ErrorCode::OutOfBoundsI8,
        // )?;
        // visitor.visit_i8(f as i8)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // let f = self.parse_int()?;
        // if f < std::i16::MIN as i64 || f > std::i16::MAX as i64 {
        //     self.fail(ErrorCode::OutOfBoundsI16)
        // } else {
        //     visitor.visit_i16(f as i16)
        // }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // let f = self.parse_int()?;
        // if f < std::i32::MIN as i64 || f > std::i32::MAX as i64 {
        //     self.fail(ErrorCode::OutOfBoundsI32)
        // } else {
        //     visitor.visit_i32(f as i32)
        // }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // let f = self.parse_int()?;
        // visitor.visit_i64(f)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // let f = self.parse_int()?;
        // if f < 0 || f > std::u8::MAX as i64 {
        //     self.fail(ErrorCode::OutOfBoundsU8)
        // } else {
        //     visitor.visit_u8(f as u8)
        // }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // let f = self.parse_int()?;
        // if f < 0 || f > std::u16::MAX as i64 {
        //     self.fail(ErrorCode::OutOfBoundsU16)
        // } else {
        //     visitor.visit_u16(f as u16)
        // }
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // let f = self.parse_int()?;
        // if f < 0 || f > std::u32::MAX as i64 {
        //     self.fail(ErrorCode::OutOfBoundsU32)
        // } else {
        //     visitor.visit_u32(f as u32)
        // }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // let f = self.parse_int()?;
        // if f < 0 {
        //     self.fail(ErrorCode::OutOfBoundsU64)
        // } else {
        //     visitor.visit_u64(f as u64)
        // }
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













































// /// Error code and byte offset describing a deserialization failure
// #[derive(PartialEq, Eq, Debug, Clone)]
// pub struct DecodeHumanReadableError<L> {
//     /// Reason decoding failed
//     pub code: ErrorCode<L>,
//
//     /// Byte offset at which the decoding failure occurred
//     pub position: usize,
// }
//
// /// Everything that can go wrong during deserialization.
// #[derive(PartialEq, Eq, Debug, Clone)]
// pub enum ErrorCode<L> {
//     /// Expected more data but the input ended.
//     UnexpectedEndOfInput,
//     /// Reached the last item of the input producer.
//     Last(L),
//     /// A generic syntax error. Any valid human-readable vv would have been ok, but alas...
//     // Syntax,
//     // /// Expected a comma (`,`) to separate collection elements.
//     // Comma,
//     // /// Expected a colon (`:`) to separate a key from a value.
//     // Colon,
//     // /// Expected a decimal digit. Didn't get one. Sad times.
//     // Digit,
//     // /// Expected hexadecimal digit as part of a unicode escape sequence in a string.
//     // HexDigit,
//     // /// Expected a unicode escape (because we just parsed a unicode escape of a leading
//     // /// surrogate codepoint).
//     // UnicodeEscape,
//     // /// Could not merge two unicode escapes into a single code point.
//     // SurrogatePair(InvalidUtf16Tuple),
//     // /// A unicode escape encoded a trailing surrogate codepoint without a preceding
//     // /// leading surrogate codepoint.
//     // TrailingSurrogate,
//     // /// A string contained an unescaped control code point.
//     // UnescapedControlCodePoint,
//     // /// A string contained a backslash followed by a non-escape character.
//     // InvalidEscape,
//     // /// A string literal contains a non-utf8 byte sequence.
//     // InvalidUtf8String,
//     // /// A number is valid json but it evaluates to -0 or an infinity
//     // InvalidNumber,
//     // /// The input contained valid json followed by at least one non-whitespace byte.
//     // TrailingCharacters,
//     // /// Attempted to parse a number as an `i8` that was out of bounds.
//     // OutOfBoundsI8,
//     // /// Attempted to parse a number as an `i16` that was out of bounds.
//     // OutOfBoundsI16,
//     // /// Attempted to parse a number as an `i32` that was out of bounds.
//     // OutOfBoundsI32,
//     // /// Attempted to parse a number as an `i64` that was less than -2^53 or greater than 2^53.
//     // OutOfBoundsI64,
//     // /// Attempted to parse a number as an `u8` that was out of bounds.
//     // OutOfBoundsU8,
//     // /// Attempted to parse a number as an `u16` that was out of bounds.
//     // OutOfBoundsU16,
//     // /// Attempted to parse a number as an `u32` that was out of bounds.
//     // OutOfBoundsU32,
//     // /// Attempted to parse a number as an `u64` that was greater than 2^53.
//     // OutOfBoundsU64,
//     // /// Chars are represented as strings that contain one unicode scalar value.
//     // NotAChar,
//     // /// Attempted to read a string as base64-encoded bytes, but the string was not valid base64.
//     // Base64(base64::DecodeError),
//     // /// Expected a boolean, found something else.
//     // ExpectedBool,
//     // /// Expected a number, found something else.
//     // ExpectedNumber,
//     // /// Expected a string, found something else.
//     // ExpectedString,
//     // /// Expected null, found something else.
//     // ExpectedNil,
//     // /// Expected an array, found something else.
//     // ExpectedArray,
//     // /// Expected an object, found something else.
//     // ExpectedObject,
//     // /// Expected an enum, found something else.
//     // ExpectedEnum,
//     /// Custom, stringly-typed error.
//     Message(String),
// }
//
// impl<L: std::fmt::Debug> fmt::Display for DecodeHumanReadableError<L> {
//     fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
//         fmt::Debug::fmt(&self.code, f)
//     }
// }
//
// impl<L: std::fmt::Debug> error::Error for DecodeHumanReadableError<L> {}
//
// impl<L: std::fmt::Debug> de::Error for DecodeHumanReadableError<L> {
//     fn custom<T: fmt::Display>(msg: T) -> Self {
//         DecodeHumanReadableError {
//             code: ErrorCode::Message(msg.to_string()),
//             position: 0, // TODO
//         }
//     }
// }
//
// type Result<T, L> = core::result::Result<T, DecodeHumanReadableError<L>>;
//
// pub struct Deserializer<P> {
//     producer: P,
// }
//
// impl<P> Deserializer<P> {
//     pub fn from_bulk_producer(producer: P) -> Self {
//         Deserializer { producer }
//     }
// }
//
// pub fn from_bulk_producer<T, P, L>(producer: P) -> Result<T, L>
// where
//     T: DeserializeOwned,
//     P: BulkProducer<Repeated = u8, Last = L>,
// {
//     let mut deserializer = Deserializer::from_bulk_producer(producer);
//     return T::deserialize(&mut deserializer);
// }
//
// impl<'de, P> Deserializer<P> {
//     // // Look at the first character in the input without consuming it.
//     // fn peek_char(&mut self) -> Result<char> {
//     //     self.input.chars().next().ok_or(Error::Eof)
//     // }
//     //
//     // // Consume the first character in the input.
//     // fn next_char(&mut self) -> Result<char> {
//     //     let ch = self.peek_char()?;
//     //     self.input = &self.input[ch.len_utf8()..];
//     //     Ok(ch)
//     // }
//     //
//     // // Parse the JSON identifier `true` or `false`.
//     // fn parse_bool(&mut self) -> Result<bool> {
//     //     if self.input.starts_with("true") {
//     //         self.input = &self.input["true".len()..];
//     //         Ok(true)
//     //     } else if self.input.starts_with("false") {
//     //         self.input = &self.input["false".len()..];
//     //         Ok(false)
//     //     } else {
//     //         Err(Error::ExpectedBoolean)
//     //     }
//     // }
//     //
//     // // Parse a group of decimal digits as an unsigned integer of type T.
//     // //
//     // // This implementation is a bit too lenient, for example `001` is not
//     // // allowed in JSON. Also the various arithmetic operations can overflow and
//     // // panic or return bogus data. But it is good enough for example code!
//     // fn parse_unsigned<T>(&mut self) -> Result<T>
//     // where
//     //     T: AddAssign<T> + MulAssign<T> + From<u8>,
//     // {
//     //     let mut int = match self.next_char()? {
//     //         ch @ '0'..='9' => T::from(ch as u8 - b'0'),
//     //         _ => {
//     //             return Err(Error::ExpectedInteger);
//     //         }
//     //     };
//     //     loop {
//     //         match self.input.chars().next() {
//     //             Some(ch @ '0'..='9') => {
//     //                 self.input = &self.input[1..];
//     //                 int *= T::from(10);
//     //                 int += T::from(ch as u8 - b'0');
//     //             }
//     //             _ => {
//     //                 return Ok(int);
//     //             }
//     //         }
//     //     }
//     // }
//     //
//     // // Parse a possible minus sign followed by a group of decimal digits as a
//     // // signed integer of type T.
//     // fn parse_signed<T>(&mut self) -> Result<T>
//     // where
//     //     T: Neg<Output = T> + AddAssign<T> + MulAssign<T> + From<i8>,
//     // {
//     //     // Optional minus sign, delegate to `parse_unsigned`, negate if negative.
//     //     unimplemented!()
//     // }
//     //
//     // // Parse a string until the next '"' character.
//     // //
//     // // Makes no attempt to handle escape sequences. What did you expect? This is
//     // // example code!
//     // fn parse_string(&mut self) -> Result<&'de str> {
//     //     if self.next_char()? != '"' {
//     //         return Err(Error::ExpectedString);
//     //     }
//     //     match self.input.find('"') {
//     //         Some(len) => {
//     //             let s = &self.input[..len];
//     //             self.input = &self.input[len + 1..];
//     //             Ok(s)
//     //         }
//     //         None => Err(Error::Eof),
//     //     }
//     // }
// }
//
// // impl<'de, P, L> de::Deserializer<'de> for &mut Deserializer<P>
// // where
// //     P: BulkProducer<Repeated = u8, Last = L>,
// // {
// //     type Error = Error<L>;
// //
// //     // Look at the input data to decide what Serde data model type to
// //     // deserialize as. Not all data formats are able to support this operation.
// //     // Formats that support `deserialize_any` are known as self-describing.
// //     fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // match self.peek_char()? {
// //         //     'n' => self.deserialize_unit(visitor),
// //         //     't' | 'f' => self.deserialize_bool(visitor),
// //         //     '"' => self.deserialize_str(visitor),
// //         //     '0'..='9' => self.deserialize_u64(visitor),
// //         //     '-' => self.deserialize_i64(visitor),
// //         //     '[' => self.deserialize_seq(visitor),
// //         //     '{' => self.deserialize_map(visitor),
// //         //     _ => Err(Error::Syntax),
// //         // }
// //     }
// //
// //     // Uses the `parse_bool` parsing function defined above to read the JSON
// //     // identifier `true` or `false` from the input.
// //     //
// //     // Parsing refers to looking at the input and deciding that it contains the
// //     // JSON value `true` or `false`.
// //     //
// //     // Deserialization refers to mapping that JSON value into Serde's data
// //     // model by invoking one of the `Visitor` methods. In the case of JSON and
// //     // bool that mapping is straightforward so the distinction may seem silly,
// //     // but in other cases Deserializers sometimes perform non-obvious mappings.
// //     // For example the TOML format has a Datetime type and Serde's data model
// //     // does not. In the `toml` crate, a Datetime in the input is deserialized by
// //     // mapping it to a Serde data model "struct" type with a special name and a
// //     // single field containing the Datetime represented as a string.
// //     fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // visitor.visit_bool(self.parse_bool()?)
// //     }
// //
// //     // The `parse_signed` function is generic over the integer type `T` so here
// //     // it is invoked with `T=i8`. The next 8 methods are similar.
// //     fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // visitor.visit_i8(self.parse_signed()?)
// //     }
// //
// //     fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // visitor.visit_i16(self.parse_signed()?)
// //     }
// //
// //     fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // visitor.visit_i32(self.parse_signed()?)
// //     }
// //
// //     fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // visitor.visit_i64(self.parse_signed()?)
// //     }
// //
// //     fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // visitor.visit_u8(self.parse_unsigned()?)
// //     }
// //
// //     fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // visitor.visit_u16(self.parse_unsigned()?)
// //     }
// //
// //     fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // visitor.visit_u32(self.parse_unsigned()?)
// //     }
// //
// //     fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // visitor.visit_u64(self.parse_unsigned()?)
// //     }
// //
// //     fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //     }
// //
// //     fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //     }
// //
// //     fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         // Parse a string, check that it is one character, call `visit_char`.
// //         unimplemented!()
// //     }
// //
// //     fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // visitor.visit_borrowed_str(self.parse_string()?)
// //     }
// //
// //     fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         self.deserialize_str(visitor)
// //     }
// //
// //     fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //     }
// //
// //     fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //     }
// //
// //     // An absent optional is represented as the JSON `null` and a present
// //     // optional is represented as just the contained value.
// //     //
// //     // As commented in `Serializer` implementation, this is a lossy
// //     // representation. For example the values `Some(())` and `None` both
// //     // serialize as just `null`. Unfortunately this is typically what people
// //     // expect when working with JSON. Other formats are encouraged to behave
// //     // more intelligently if possible.
// //     fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // if self.input.starts_with("null") {
// //         //     self.input = &self.input["null".len()..];
// //         //     visitor.visit_none()
// //         // } else {
// //         //     visitor.visit_some(self)
// //         // }
// //     }
// //
// //     // In Serde, unit means an anonymous value containing no data.
// //     fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // if self.input.starts_with("null") {
// //         //     self.input = &self.input["null".len()..];
// //         //     visitor.visit_unit()
// //         // } else {
// //         //     Err(Error::ExpectedNil)
// //         // }
// //     }
// //
// //     // Unit struct means a named value containing no data.
// //     fn deserialize_unit_struct<V>(
// //         self,
// //         _name: &'static str,
// //         visitor: V,
// //     ) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         self.deserialize_unit(visitor)
// //     }
// //
// //     fn deserialize_newtype_struct<V>(
// //         self,
// //         _name: &'static str,
// //         visitor: V,
// //     ) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         visitor.visit_newtype_struct(self)
// //     }
// //
// //     // Deserialization of compound types like sequences and maps happens by
// //     // passing the visitor an "Access" object that gives it the ability to
// //     // iterate through the data contained in the sequence.
// //     fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // // Parse the opening bracket of the sequence.
// //         // if self.next_char()? == '[' {
// //         //     // Give the visitor access to each element of the sequence.
// //         //     let value = visitor.visit_seq(CommaSeparated::new(&mut self))?;
// //         //     // Parse the closing bracket of the sequence.
// //         //     if self.next_char()? == ']' {
// //         //         Ok(value)
// //         //     } else {
// //         //         Err(Error::ExpectedArrayEnd)
// //         //     }
// //         // } else {
// //         //     Err(Error::ExpectedArray)
// //         // }
// //     }
// //
// //     fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         self.deserialize_seq(visitor)
// //     }
// //
// //
// //     // Tuple structs look just like sequences in JSON.
// //     fn deserialize_tuple_struct<V>(
// //         self,
// //         _name: &'static str,
// //         _len: usize,
// //         visitor: V,
// //     ) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         self.deserialize_seq(visitor)
// //     }
// //
// //     // Much like `deserialize_seq` but calls the visitors `visit_map` method
// //     // with a `MapAccess` implementation, rather than the visitor's `visit_seq`
// //     // method with a `SeqAccess` implementation.
// //     fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // // Parse the opening brace of the map.
// //         // if self.next_char()? == '{' {
// //         //     // Give the visitor access to each entry of the map.
// //         //     let value = visitor.visit_map(CommaSeparated::new(&mut self))?;
// //         //     // Parse the closing brace of the map.
// //         //     if self.next_char()? == '}' {
// //         //         Ok(value)
// //         //     } else {
// //         //         Err(Error::ExpectedMapEnd)
// //         //     }
// //         // } else {
// //         //     Err(Error::ExpectedMap)
// //         // }
// //     }
// //
// //     // Structs look just like maps in JSON.
// //     //
// //     // Notice the `fields` parameter - a "struct" in the Serde data model means
// //     // that the `Deserialize` implementation is required to know what the fields
// //     // are before even looking at the input data. Any key-value pairing in which
// //     // the fields cannot be known ahead of time is probably a map.
// //     fn deserialize_struct<V>(
// //         self,
// //         _name: &'static str,
// //         _fields: &'static [&'static str],
// //         visitor: V,
// //     ) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         self.deserialize_map(visitor)
// //     }
// //
// //     fn deserialize_enum<V>(
// //         self,
// //         _name: &'static str,
// //         _variants: &'static [&'static str],
// //         visitor: V,
// //     ) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // if self.peek_char()? == '"' {
// //         //     // Visit a unit variant.
// //         //     visitor.visit_enum(self.parse_string()?.into_deserializer())
// //         // } else if self.next_char()? == '{' {
// //         //     // Visit a newtype variant, tuple variant, or struct variant.
// //         //     let value = visitor.visit_enum(Enum::new(self))?;
// //         //     // Parse the matching close brace.
// //         //     if self.next_char()? == '}' {
// //         //         Ok(value)
// //         //     } else {
// //         //         Err(Error::ExpectedMapEnd)
// //         //     }
// //         // } else {
// //         //     Err(Error::ExpectedEnum)
// //         // }
// //     }
// //
// //     // An identifier in Serde is the type that identifies a field of a struct or
// //     // the variant of an enum. In JSON, struct fields and enum variants are
// //     // represented as strings. In other formats they may be represented as
// //     // numeric indices.
// //     fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         unimplemented!()
// //         // self.deserialize_str(visitor)
// //     }
// //
// //     // Like `deserialize_any` but indicates to the `Deserializer` that it makes
// //     // no difference which `Visitor` method is called because the data is
// //     // ignored.
// //     //
// //     // Some deserializers are able to implement this more efficiently than
// //     // `deserialize_any`, for example by rapidly skipping over matched
// //     // delimiters without paying close attention to the data in between.
// //     //
// //     // Some formats are not able to implement this at all. Formats that can
// //     // implement `deserialize_any` and `deserialize_ignored_any` are known as
// //     // self-describing.
// //     fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, L>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         self.deserialize_any(visitor)
// //     }
// // }
// //
// // // In order to handle commas correctly when deserializing a JSON array or map,
// // // we need to track whether we are on the first element or past the first
// // // element.
// // struct CommaSeparated<'a, P> {
// //     de: &'a mut Deserializer<P>,
// //     first: bool,
// // }
// //
// // impl<'a, P> CommaSeparated<'a, P> {
// //     fn new(de: &'a mut Deserializer<P>) -> Self {
// //         CommaSeparated {
// //             de,
// //             first: true,
// //         }
// //     }
// // }
// //
// // // `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
// // // through elements of the sequence.
// // impl<'a, P, L> SeqAccess<'_> for CommaSeparated<'a, P> {
// //     type Error = Error<L>;
// //
// //     fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<<T as serde::de::DeserializeSeed<'_>>::Value>, L>
// //     where
// //         for<'de> T: DeserializeSeed<'de>,
// //     {
// //         unimplemented!()
// //         // // Check if there are no more elements.
// //         // if self.de.peek_char()? == ']' {
// //         //     return Ok(None);
// //         // }
// //         // // Comma is required before every element except the first.
// //         // if !self.first && self.de.next_char()? != ',' {
// //         //     return Err(Error::ExpectedArrayComma);
// //         // }
// //         // self.first = false;
// //         // // Deserialize an array element.
// //         // seed.deserialize(&mut *self.de).map(Some)
// //     }
// // }
// //
// // // `MapAccess` is provided to the `Visitor` to give it the ability to iterate
// // // through entries of the map.
// // impl<'a, P, L> MapAccess<'_> for CommaSeparated<'a, P> {
// //     type Error = Error<L>;
// //
// //     fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<<K as serde::de::DeserializeSeed<'_>>::Value>, L>
// //     where
// //         for<'de> K: DeserializeSeed<'de>,
// //     {
// //         unimplemented!()
// //         // // Check if there are no more entries.
// //         // if self.de.peek_char()? == '}' {
// //         //     return Ok(None);
// //         // }
// //         // // Comma is required before every entry except the first.
// //         // if !self.first && self.de.next_char()? != ',' {
// //         //     return Err(Error::ExpectedMapComma);
// //         // }
// //         // self.first = false;
// //         // // Deserialize a map key.
// //         // seed.deserialize(&mut *self.de).map(Some)
// //     }
// //
// //     fn next_value_seed<V>(&mut self, seed: V) -> Result<<V as serde::de::DeserializeSeed<'_>>::Value, L>
// //     where
// //         for<'de> V: DeserializeSeed<'de>,
// //     {
// //         unimplemented!()
// //         // // It doesn't make a difference whether the colon is parsed at the end
// //         // // of `next_key_seed` or at the beginning of `next_value_seed`. In this
// //         // // case the code is a bit simpler having it here.
// //         // if self.de.next_char()? != ':' {
// //         //     return Err(Error::ExpectedMapColon);
// //         // }
// //         // // Deserialize a map value.
// //         // seed.deserialize(&mut *self.de)
// //     }
// // }
// //
// // struct Enum<'a, P> {
// //     de: &'a mut Deserializer<P>,
// // }
// //
// // impl<'a, P> Enum<'a, P> {
// //     fn new(de: &'a mut Deserializer<P>) -> Self {
// //         Enum { de }
// //     }
// // }
// //
// // // `EnumAccess` is provided to the `Visitor` to give it the ability to determine
// // // which variant of the enum is supposed to be deserialized.
// // //
// // // Note that all enum deserialization methods in Serde refer exclusively to the
// // // "externally tagged" enum representation.
// // impl<'a, P, L> EnumAccess<'_> for Enum<'a, P> {
// //     type Error = Error<L>;
// //     type Variant = Self;
// //
// //     fn variant_seed<V>(self, seed: V) -> Result<(<V as serde::de::DeserializeSeed<'_>>::Value, Self::Variant), L>
// //     where
// //         for<'de> V: DeserializeSeed<'de>,
// //     {
// //         unimplemented!()
// //         // // The `deserialize_enum` method parsed a `{` character so we are
// //         // // currently inside of a map. The seed will be deserializing itself from
// //         // // the key of the map.
// //         // let val = seed.deserialize(&mut *self.de)?;
// //         // // Parse the colon separating map key from value.
// //         // if self.de.next_char()? == ':' {
// //         //     Ok((val, self))
// //         // } else {
// //         //     Err(Error::ExpectedMapColon)
// //         // }
// //     }
// // }
// //
// // // `VariantAccess` is provided to the `Visitor` to give it the ability to see
// // // the content of the single variant that it decided to deserialize.
// // impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
// //     type Error = Error;
// //
// //     // If the `Visitor` expected this variant to be a unit variant, the input
// //     // should have been the plain string case handled in `deserialize_enum`.
// //     fn unit_variant(self) -> Result<()> {
// //         Err(Error::ExpectedString)
// //     }
// //
// //     // Newtype variants are represented in JSON as `{ NAME: VALUE }` so
// //     // deserialize the value here.
// //     fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
// //     where
// //         T: DeserializeSeed<'de>,
// //     {
// //         seed.deserialize(self.de)
// //     }
// //
// //     // Tuple variants are represented in JSON as `{ NAME: [DATA...] }` so
// //     // deserialize the sequence of data here.
// //     fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         de::Deserializer::deserialize_seq(self.de, visitor)
// //     }
// //
// //     // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }` so
// //     // deserialize the inner map here.
// //     fn struct_variant<V>(
// //         self,
// //         _fields: &'static [&'static str],
// //         visitor: V,
// //     ) -> Result<V::Value>
// //     where
// //         V: Visitor<'de>,
// //     {
// //         de::Deserializer::deserialize_map(self.de, visitor)
// //     }
// // }
//
// ////////////////////////////////////////////////////////////////////////////////
//
// // #[test]
// // fn test_struct() {
// //     #[derive(Deserialize, PartialEq, Debug)]
// //     struct Test {
// //         int: u32,
// //         seq: Vec<String>,
// //     }
// //
// //     let j = r#"{"int":1,"seq":["a","b"]}"#;
// //     let expected = Test {
// //         int: 1,
// //         seq: vec!["a".to_owned(), "b".to_owned()],
// //     };
// //     assert_eq!(expected, from_str(j).unwrap());
// // }
// //
// // #[test]
// // fn test_enum() {
// //     #[derive(Deserialize, PartialEq, Debug)]
// //     enum E {
// //         Unit,
// //         Newtype(u32),
// //         Tuple(u32, u32),
// //         Struct { a: u32 },
// //     }
// //
// //     let j = r#""Unit""#;
// //     let expected = E::Unit;
// //     assert_eq!(expected, from_str(j).unwrap());
// //
// //     let j = r#"{"Newtype":1}"#;
// //     let expected = E::Newtype(1);
// //     assert_eq!(expected, from_str(j).unwrap());
// //
// //     let j = r#"{"Tuple":[1,2]}"#;
// //     let expected = E::Tuple(1, 2);
// //     assert_eq!(expected, from_str(j).unwrap());
// //
// //     let j = r#"{"Struct":{"a":1}}"#;
// //     let expected = E::Struct { a: 1 };
// //     assert_eq!(expected, from_str(j).unwrap());
// // }
