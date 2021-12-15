use serde::Deserialize;
use std::str::FromStr;
use std::fmt;

use thiserror::Error;
use atm_parser_helper::{ParserHelper, Eoi, Error as ParseError};
use atm_parser_helper_common_syntax::*;

use serde::de::{
    self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor,
};

use crate::always_nil::AlwaysNil;

/// Everything that can go wrong during deserialization of a valuable value from the human-readable
/// encoding.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum DecodeError {
    /// Unexpectedly reached the end of the input.
    #[error("unexpected end of input")]
    Eoi,
    /// Custom, stringly-typed error, used by serde.
    #[error("{0}")]
    Message(String),
    #[error("invalid syntax, not a valuable value")]
    Syntax,

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
    /// Attempted to parse a number as an `char` that was out of bounds.
    #[error("char out of bounds")]
    OutOfBoundsChar,
    #[error("string byte count may not exceed 2^63 - 1")]
    OutOfBoundsString,
    #[error("array count may not exceed 2^63 - 1")]
    OutOfBoundsArray,
    #[error("set count may not exceed 2^63 - 1")]
    OutOfBoundsSet,
    #[error("map count may not exceed 2^63 - 1")]
    OutOfBoundsMap,

    #[error("can only decode a set where a map whose values are all nil would be valid")]
    InvalidSet,

    #[error("expected nil")]
    ExpectedNil,
    #[error("expected bool")]
    ExpectedBool,
    #[error("expected float")]
    ExpectedFloat,
    #[error("expected int")]
    ExpectedInt,
    #[error("expected option")]
    ExpectedOption,
    #[error("expected byte string")]
    ExpectedString,
    #[error("expected byte string")]
    ExpectedBytes,
    #[error("expected array")]
    ExpectedArray,
    #[error("expected map")]
    ExpectedMap,
    #[error("expected `{0}` enum value")]
    ExpectedEnum(String),
    #[error("expected enum variant (either a string or a singleton map)")]
    ExpectedEnumVariant,

    #[error("comments must be valid UTF-8")]
    CommentUtf8,

    #[error("integer literals must have at least one digit")]
    IntDigits,

    #[error("floating-point literals must have at least one digit before the decimal point")]
    FloatLeadingDigits,
    #[error("floating-point literals must have a decimal point")]
    FloatPoint,
    #[error("floating-point literals must have at least one digit after the decimal point")]
    FloatTrailingDigits,
    #[error("floating-point literals with an exponent must have at least one exponent digit")]
    FloatExponentDigit,

    #[error("hexadecimal byte string literals must have an even number of digits")]
    ByteStringHexOdd,
    #[error("binary byte string literals must have a number of digits divisible by eight")]
    ByteStringBinaryNumber,
    #[error("the bytes of a byte string literals must be integers between 0 and 255")]
    ByteOutOfBounds,

    #[error("utf8 string literals must be valid UTF-8")]
    Utf8StringUtf8,
    #[error("raw utf8 string literals must start with at most 255 @s")]
    Utf8StringRawAts,
    #[error("invalid escape sequence")]
    Utf8StringEscape,
    #[error("unicode escapes must consist of one to six digits")]
    UnicodeDigits,
    #[error("unicode escapes must encode unicode scalar values")]
    UnicodeScalar,
    #[error("unicode escapes must be terminated by a closing brace")]
    UnicodeClosing,

    #[error("expected a comma to separate collection elements")]
    ExpectedComma,
    #[error("empty collections may not contain a comma")]
    EmptyCollectionComma,
    #[error("expected a colon after the key")]
    ExpectedColon,

    #[error("chars must be encoded as UTF-8 strings containing exactly one unicode codepoint")]
    CharLength,
}

impl Eoi for DecodeError {
    fn eoi() -> Self {
        Self::Eoi
    }
}

impl WhiteSpaceE for DecodeError {
    fn utf8_comment() -> Self {
        Self::CommentUtf8
    }
}

impl IntLiteralE for DecodeError {
    fn int_no_digits() -> Self {
        Self::IntDigits
    }

    fn not_int_literal() -> Self {
        Self::ExpectedInt
    }
}

impl FloatLiteralE for DecodeError {
    fn float_no_leading_digits() -> Self {
        Self::FloatLeadingDigits
    }

    fn float_no_point() -> Self {
        Self::FloatPoint
    }

    fn float_no_trailing_digits() -> Self {
        Self::FloatTrailingDigits
    }

    fn float_no_exponent_digits() -> Self {
        Self::FloatExponentDigit
    }

    fn not_float_literal() -> Self {
        Self::ExpectedFloat
    }
}

impl ByteStringLiteralE for DecodeError {
    fn odd_hex_digits() -> Self {
        Self::ByteStringHexOdd
    }

    fn number_binary_digits() -> Self {
        Self::ByteStringBinaryNumber
    }

    fn expected_comma() -> Self {
        Self::ExpectedComma
    }

    fn byte_out_of_bounds() -> Self {
        Self::ByteOutOfBounds
    }

    fn not_byte_string_literal() -> Self {
        Self::ExpectedBytes
    }
}

impl Utf8StringLiteralE for DecodeError {
    fn raw_not_utf8() -> Self {
        Self::Utf8StringUtf8
    }

    fn raw_too_many_ats() -> Self {
        Self::Utf8StringRawAts
    }

    fn escaping_not_utf8() -> Self {
        Self::Utf8StringUtf8
    }

    fn invalid_escape_sequence() -> Self {
        Self::Utf8StringEscape
    }

    fn unicode_escape_number_digits() -> Self {
        Self::UnicodeDigits
    }

    fn unicode_escape_invalid_scalar() -> Self {
        Self::UnicodeScalar
    }

    fn unicode_escape_no_closing() -> Self {
        Self::UnicodeClosing
    }

    fn not_utf8_string_literal() -> Self {
        Self::ExpectedString
    }
}

impl de::Error for DecodeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        DecodeError::Message(msg.to_string())
    }
}

pub type Error = ParseError<DecodeError>;

/// A structure that deserializes valuable values.
///
/// https://github.com/AljoschaMeyer/valuable-value/blob/main/README.md
pub struct VVDeserializer<'de> {
    p: ParserHelper<'de>,
}

impl<'de> VVDeserializer<'de> {
    pub fn new(input: &'de [u8]) -> Self {
        VVDeserializer {
            p: ParserHelper::new(input),
        }
    }

    pub fn position(&self) -> usize {
        self.p.position()
    }

    fn parse_nil(&mut self) -> Result<(), Error> {
        self.p.expect_bytes(b"nil", DecodeError::ExpectedNil)
    }

    fn parse_bool(&mut self) -> Result<bool, Error> {
        if self.p.advance_over(b"false") {
            Ok(false)
        } else {
            self.p.expect_bytes(b"true", DecodeError::ExpectedBool)?;
            Ok(true)
        }
    }
}

fn i64_from_decimal(s: &str) -> Result<i64, DecodeError> {
    i64::from_str_radix(s, 10).map_err(|_| DecodeError::OutOfBoundsI64)
}

fn i64_from_hex(s: &str) -> Result<i64, DecodeError> {
    i64::from_str_radix(s, 16).map_err(|_| DecodeError::OutOfBoundsI64)
}

fn i64_from_binary(s: &str) -> Result<i64, DecodeError> {
    i64::from_str_radix(s, 2).map_err(|_| DecodeError::OutOfBoundsI64)
}

fn f64_from_s(s: &str) -> Result<f64, DecodeError> {
    f64::from_str(s).map_err(|_| panic!())
}

impl<'a, 'de> de::Deserializer<'de> for &'a mut VVDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        spaces(&mut self.p)?;
        match self.p.peek()? {
            0x6e => {
                self.parse_nil()?;
                visitor.visit_unit()
            }
            0x66 | 0x74 => self.deserialize_bool(visitor),
            0x30..=0x39 | 0x2b | 0x2d | 0x49 | 0x4e => {
                match parse_number(&mut self.p, i64_from_decimal, i64_from_hex, i64_from_binary, f64_from_s, f64::NEG_INFINITY, f64::INFINITY, f64::from_bits(u64::MAX))? {
                    Number::Float(f) => visitor.visit_f64(f),
                    Number::Integer(n) => visitor.visit_i64(n),
                }
            }
            0x22 => self.deserialize_str(visitor),
            0x5b => self.deserialize_seq(visitor),
            0x7b => self.deserialize_map(visitor),
            0x40 => {
                match self.p.rest().get(1) {
                    None => self.p.fail(DecodeError::Eoi),
                    Some(0x5b | 0x62 | 0x78) => self.deserialize_bytes(visitor),
                    Some(0x22 | 0x40) => self.deserialize_str(visitor),
                    Some(0x7b) => self.deserialize_map(visitor),
                    Some(_) => self.p.fail(DecodeError::Syntax),
                }
            }
            _ => self.p.fail(DecodeError::Syntax),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        spaces(&mut self.p)?;
        visitor.visit_bool(self.parse_bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        spaces(&mut self.p)?;
        let start = self.p.position();
        let n = parse_int(&mut self.p, i64_from_decimal, i64_from_hex, i64_from_binary)?;
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
        spaces(&mut self.p)?;
        let start = self.p.position();
        let n = parse_int(&mut self.p, i64_from_decimal, i64_from_hex, i64_from_binary)?;
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
        spaces(&mut self.p)?;
        let start = self.p.position();
        let n = parse_int(&mut self.p, i64_from_decimal, i64_from_hex, i64_from_binary)?;
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
        spaces(&mut self.p)?;
        visitor.visit_i64(parse_int(&mut self.p, i64_from_decimal, i64_from_hex, i64_from_binary)?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        spaces(&mut self.p)?;
        let start = self.p.position();
        let n = parse_int(&mut self.p, i64_from_decimal, i64_from_hex, i64_from_binary)?;
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
        spaces(&mut self.p)?;
        let start = self.p.position();
        let n = parse_int(&mut self.p, i64_from_decimal, i64_from_hex, i64_from_binary)?;
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
        spaces(&mut self.p)?;
        let start = self.p.position();
        let n = parse_int(&mut self.p, i64_from_decimal, i64_from_hex, i64_from_binary)?;
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
        spaces(&mut self.p)?;
        let start = self.p.position();
        let n = parse_int(&mut self.p, i64_from_decimal, i64_from_hex, i64_from_binary)?;
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
        spaces(&mut self.p)?;
        visitor.visit_f64(parse_float(&mut self.p, f64_from_s, f64::NEG_INFINITY, f64::INFINITY, f64::from_bits(u64::MAX))?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        spaces(&mut self.p)?;
        visitor.visit_f64(parse_float(&mut self.p, f64_from_s, f64::NEG_INFINITY, f64::INFINITY, f64::from_bits(u64::MAX))?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        spaces(&mut self.p)?;
        let s = parse_utf8_string(&mut self.p)?;
        let mut cs = s.chars();
        match cs.next() {
            None => self.p.fail(DecodeError::CharLength),
            Some(c) => {
                if cs.next().is_some() {
                    self.p.fail(DecodeError::CharLength)
                } else {
                    visitor.visit_char(c)
                }
            }
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        spaces(&mut self.p)?;
        let s = parse_utf8_string(&mut self.p)?;
        visitor.visit_str(&s)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        spaces(&mut self.p)?;
        let s = parse_utf8_string(&mut self.p)?;
        visitor.visit_string(s)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        spaces(&mut self.p)?;
        let b = parse_byte_string(&mut self.p)?;
        visitor.visit_bytes(&b)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        spaces(&mut self.p)?;
        let b = parse_byte_string(&mut self.p)?;
        visitor.visit_byte_buf(b)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // if self.p.advance_over(&[0b100_00100, 'N' as u8, 'o' as u8, 'n' as u8, 'e' as u8]) || (self.canonic && self.p.advance_over(&[0b101_00100, 0b011_11100, 'N' as u8, 0b011_11100, 'o' as u8, 0b011_11100, 'n' as u8, 0b011_11100, 'e' as u8])) {
        //     return visitor.visit_none();
        // } else if self.p.advance_over(&[0b111_00001, 0b100_00100, 'S' as u8, 'o' as u8, 'm' as u8, 'e' as u8]) || (self.canonic && self.p.advance_over(&[0b111_00001, 0b101_00100, 0b011_11100, 'S' as u8, 0b011_11100, 'o' as u8, 0b011_11100, 'm' as u8, 0b011_11100, 'e' as u8])) {
        //     return visitor.visit_some(self);
        // } else {
        //     return self.p.fail(DecodeError::ExpectedOption);
        // }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        spaces(&mut self.p)?;
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
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        spaces(&mut self.p)?;
        self.p.expect('[' as u8, DecodeError::ExpectedArray)?;
        return visitor.visit_seq(SequenceAccessor::new(&mut self));
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
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
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        spaces(&mut self.p)?;
        if self.p.advance_over(b"@{") {
            return visitor.visit_map(MapAccessor::new(&mut self, true));
        } else if self.p.advance_over(b"{") {
            return visitor.visit_map(MapAccessor::new(&mut self, false));
        } else {
            return self.p.fail(DecodeError::ExpectedMap);
        }
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
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
        // match self.p.peek()? & 0b111_00000 {
        //     0b100_00000 | 0b110_00000 | 0b111_00000 => Ok(visitor.visit_enum(Enum::new(self))?),
        //     0b101_00000 if self.canonic => Ok(visitor.visit_enum(Enum::new(self))?),
        //     0b101_00000 if !self.canonic => Ok(visitor.visit_enum(Enum::new(self))?),
        //     _ => self.p.fail(DecodeError::ExpectedEnum(name.to_string()))
        // }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn is_human_readable(&self) -> bool {
        true
    }
}

struct SequenceAccessor<'a, 'de> {
    des: &'a mut VVDeserializer<'de>,
    first: bool,
}

impl<'a, 'de> SequenceAccessor<'a, 'de> {
    fn new(des: &'a mut VVDeserializer<'de>) -> SequenceAccessor<'a, 'de> {
        SequenceAccessor { des, first: true }
    }
}

impl<'a, 'de> SeqAccess<'de> for SequenceAccessor<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        spaces(&mut self.des.p)?;

        if self.des.p.advance_over(b"]") {
            return Ok(None);
        } else if self.des.p.advance_over(b",") {
            spaces(&mut self.des.p)?;
            if self.des.p.advance_over(b"]") {
                if self.first {
                    return self.des.p.fail(DecodeError::EmptyCollectionComma);
                } else {
                    return Ok(None);
                }
            } else {
                return Ok(Some(seed.deserialize(&mut *self.des)?));
            }
        } else if !self.first {
            return self.des.p.fail(DecodeError::ExpectedComma);
        } else {
            self.first = false;
            return Ok(Some(seed.deserialize(&mut *self.des)?));
        }
    }
}

struct MapAccessor<'a, 'de> {
    des: &'a mut VVDeserializer<'de>,
    set: bool,
    first: bool,
}

impl<'a, 'de> MapAccessor<'a, 'de> {
    fn new(des: &'a mut VVDeserializer<'de>, set: bool) -> MapAccessor<'a, 'de> {
        MapAccessor { des, set, first: true }
    }
}

impl<'a, 'de> MapAccess<'de> for MapAccessor<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        spaces(&mut self.des.p)?;

        if self.des.p.advance_over(b"}") {
            return Ok(None);
        } else if self.des.p.advance_over(b",") {
            spaces(&mut self.des.p)?;
            if self.des.p.advance_over(b"}") {
                if self.first {
                    return self.des.p.fail(DecodeError::EmptyCollectionComma);
                } else {
                    return Ok(None);
                }
            } else {
                return Ok(Some(seed.deserialize(&mut *self.des)?));
            }
        } else if !self.first {
            return self.des.p.fail(DecodeError::ExpectedComma);
        } else {
            self.first = false;
            return Ok(Some(seed.deserialize(&mut *self.des)?));
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        if self.set {
            match seed.deserialize(AlwaysNil::new()) {
                Ok(nil) => return Ok(nil),
                Err(_) => return self.des.p.fail(DecodeError::InvalidSet),
            }
        } else {
            spaces(&mut self.des.p)?;
            self.des.p.expect(':' as u8, DecodeError::ExpectedColon)?;
            spaces(&mut self.des.p)?;
            return Ok(seed.deserialize(&mut *self.des)?);
        }
    }
}

struct Enum<'a, 'de> {
    des: &'a mut VVDeserializer<'de>,
    set: bool,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(des: &'a mut VVDeserializer<'de>) -> Self {
        Enum { des, set: false }
    }
}

impl<'a, 'de> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(mut self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        match self.des.p.peek()? {
            b if (b & 0b111_00000 == 0b100_00000) || (b & 0b111_00000 == 0b101_00000) => Ok((seed.deserialize(&mut *self.des)?, self)),
            0b110_00001 => {
                self.set = true;
                self.des.p.advance(1);
                Ok((seed.deserialize(&mut *self.des)?, self))
            }
            0b111_00001 => {
                self.des.p.advance(1);
                Ok((seed.deserialize(&mut *self.des)?, self))
            }
            _ => self.des.p.fail(DecodeError::ExpectedEnumVariant),
        }
    }
}

impl<'a, 'de> VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.set {
            match seed.deserialize(AlwaysNil::new()) {
                Ok(nil) => Ok(nil),
                Err(_) => self.des.p.fail(DecodeError::InvalidSet),
            }
        } else {
            seed.deserialize(self.des)
        }
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.des, visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.des, visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats() {
        let f = f64::deserialize(&mut VVDeserializer::new(&[0b010_00000, 0x80, 0, 0, 0, 0, 0, 0, 0])).unwrap();
        assert_eq!(f, -0.0f64);
        assert!(f.is_sign_negative());
    }

    #[test]
    fn arrays() {
        let mut d = VVDeserializer::new(&[0b101_11111, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 0]);
        assert_eq!(Vec::<()>::deserialize(&mut d).unwrap_err().e, DecodeError::OutOfBoundsArray);

        let mut d = VVDeserializer::new(&[0b101_11111, 126, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 0]);
        assert_eq!(Vec::<()>::deserialize(&mut d).unwrap_err().e, DecodeError::Eoi);
    }
}
