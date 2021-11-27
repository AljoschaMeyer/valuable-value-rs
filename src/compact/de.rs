use serde::Deserialize;
use core::marker::PhantomData;
use std::convert::TryInto;
use std::fmt;

use thiserror::Error;
use atm_parser_helper::{ParserHelper, Eoi, Error as ParseError};

use serde::de::{
    self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor,
};

use crate::always_nil::AlwaysNil;

/// Everything that can go wrong during deserialization of a valuable value from the compact
/// encoding.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum DecodeError {
    /// Unexpectedly reached the end of the input.
    #[error("unexpected end of input")]
    Eoi,
    /// Custom, stringly-typed error, used by serde.
    #[error("{0}")]
    Message(String),
    /// The input was not canonic, but the deserializer was configured to reject non-canonic input.
    #[error("{0}")]
    Canonicity(CanonicityCondition),

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

    #[error("rust strings must be utf8, the input string was not")]
    Utf8,

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
}

impl Eoi for DecodeError {
    fn eoi() -> Self {
        Self::Eoi
    }
}

impl de::Error for DecodeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        DecodeError::Message(msg.to_string())
    }
}

type Error = ParseError<DecodeError>;

/// The different ways of violating canonicity.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum CanonicityCondition {
    #[error("canonicity requires that NaN is encoded as eight 0xff bytes")]
    NaN,
    #[error("canonicity requires that the integer is encoded with fewer bytes")]
    IntTooWide,
    #[error("canonicity requires that the string byte count is encoded with fewer bytes")]
    StringTooWide,
    #[error("canonicity requires that the array count is encoded with fewer bytes")]
    ArrayTooWide,
    #[error("canonicity requires that the map count is encoded with fewer bytes")]
    MapTooWide,
    #[error("canonicity requires that (byte) strings are encoded as regular arrays")]
    Bytes,
    #[error("canonicity requires that sets are encoded as regular maps")]
    Set,
}

/// A structure that deserializes valuable values.
///
/// https://github.com/AljoschaMeyer/valuable-value/blob/main/README.md
pub struct VVDeserializer<'de> {
    p: ParserHelper<'de>,
    canonic: bool,
}

impl<'de> VVDeserializer<'de> {
    pub fn new(input: &'de [u8], canonic: bool) -> Self {
        VVDeserializer {
            p: ParserHelper::new(input),
            canonic,
        }
    }

    pub fn position(&self) -> usize {
        self.p.position()
    }

    fn parse_nil(&mut self) -> Result<(), Error> {
        self.p.expect(0b000_00000, DecodeError::ExpectedNil)
    }

    fn parse_bool(&mut self) -> Result<bool, Error> {
        match self.p.next()? {
            0b001_00000 => Ok(false),
            0b001_00001 => Ok(true),
            _ => self.p.fail_at_position(DecodeError::ExpectedBool, self.p.position() - 1),
        }
    }

    fn parse_float(&mut self) -> Result<f64, Error> {
        self.p.expect(0b010_00000, DecodeError::ExpectedFloat)?;

        let start = self.p.position();
        self.p.advance_or(8, DecodeError::Eoi)?;
        let n = f64::from_bits(u64::from_be_bytes(self.p.slice(start..start + 8).try_into().unwrap()));
        if self.canonic {
            if n.to_bits() != u64::MAX {
                return self.p.fail(DecodeError::Canonicity(CanonicityCondition::NaN));
            }
        }
        return Ok(n);
    }

    fn parse_int(&mut self) -> Result<i64, Error> {
        match self.p.next()? {
            b if b & 0b111_00000 == 0b011_00000 => {
                if b == 0b011_11111 {
                    let start = self.p.position();
                    self.p.advance_or(8, DecodeError::Eoi)?;
                    let n = i64::from_be_bytes(self.p.slice(start..start + 8).try_into().unwrap());
                    if self.canonic && (i32::MIN as i64) <= n && n <= (i32::MAX as i64) {
                        return self.p.fail_at_position(DecodeError::Canonicity(CanonicityCondition::IntTooWide), start);
                    }
                    return Ok(n);
                } else if b == 0b011_11110 {
                    let start = self.p.position();
                    self.p.advance_or(4, DecodeError::Eoi)?;
                    let n = i32::from_be_bytes(self.p.slice(start..start + 4).try_into().unwrap()) as i64;
                    if self.canonic && (i16::MIN as i64) <= n && n <= (i16::MAX as i64) {
                        return self.p.fail_at_position(DecodeError::Canonicity(CanonicityCondition::IntTooWide), start);
                    }
                    return Ok(n);
                } else if b == 0b011_11101 {
                    let start = self.p.position();
                    self.p.advance_or(2, DecodeError::Eoi)?;
                    let n = i16::from_be_bytes(self.p.slice(start..start + 2).try_into().unwrap()) as i64;
                    if self.canonic && (i8::MIN as i64) <= n && n <= (i8::MAX as i64) {
                        return self.p.fail_at_position(DecodeError::Canonicity(CanonicityCondition::IntTooWide), start);
                    }
                    return Ok(n);
                } else if b == 0b011_11100 {
                    let start = self.p.position();
                    self.p.advance_or(1, DecodeError::Eoi)?;
                    let n = i8::from_be_bytes(self.p.slice(start..start + 1).try_into().unwrap()) as i64;
                    if self.canonic && 0 <= n && n <= 11 {
                        return self.p.fail_at_position(DecodeError::Canonicity(CanonicityCondition::IntTooWide), start);
                    }
                    return Ok(n);
                } else {
                    return Ok((u8::from_be_bytes([b & 0b000_11111])) as i64);
                }
            }
            _ => self.p.fail_at_position(DecodeError::ExpectedInt, self.p.position() - 1),
        }
    }

    fn parse_bytes(&mut self) -> Result<&[u8], Error> {
        if self.canonic {
            return self.p.fail(DecodeError::Canonicity(CanonicityCondition::Bytes));
        }
        let count = self.parse_count(0b100_00000, DecodeError::ExpectedBytes, CanonicityCondition::StringTooWide, DecodeError::OutOfBoundsString)?;
        let start = self.p.position();
        if self.p.rest().len() < count {
            return self.p.unexpected_end_of_input();
        } else {
            self.p.advance(count);
            return Ok(self.p.slice(start..self.p.position()));
        }
    }

    fn parse_count(&mut self, tag: u8, expected: DecodeError, too_wide: CanonicityCondition, out_of_bounds: DecodeError) -> Result<usize, Error> {
        match self.p.next()? {
            b if b & 0b111_00000 == tag => {
                let len = if b == (tag | 0b000_11111) {
                    let start = self.p.position();
                    self.p.advance_or(8, DecodeError::Eoi)?;
                    let n = u64::from_be_bytes(self.p.slice(start..start + 8).try_into().unwrap());
                    if self.canonic && n <= (u32::MAX as u64) {
                        return self.p.fail_at_position(DecodeError::Canonicity(too_wide), start);
                    }
                    if n > (i64::MAX as u64) {
                        return self.p.fail(out_of_bounds);
                    }
                    n
                } else if b == (tag | 0b000_11110) {
                    let start = self.p.position();
                    self.p.advance_or(4, DecodeError::Eoi)?;
                    let n = u32::from_be_bytes(self.p.slice(start..start + 4).try_into().unwrap()) as u64;
                    if self.canonic && n <= (u16::MAX as u64) {
                        return self.p.fail_at_position(DecodeError::Canonicity(too_wide), start);
                    }
                    n
                } else if b == (tag | 0b000_11101) {
                    let start = self.p.position();
                    self.p.advance_or(2, DecodeError::Eoi)?;
                    let n = u16::from_be_bytes(self.p.slice(start..start + 2).try_into().unwrap()) as u64;
                    if self.canonic && n <= (u8::MAX as u64) {
                        return self.p.fail_at_position(DecodeError::Canonicity(too_wide), start);
                    }
                    n
                } else if b == (tag | 0b000_11100) {
                    let start = self.p.position();
                    self.p.advance_or(1, DecodeError::Eoi)?;
                    let n = u8::from_be_bytes(self.p.slice(start..start + 1).try_into().unwrap()) as u64;
                    if self.canonic && n <= 11 {
                        return self.p.fail_at_position(DecodeError::Canonicity(too_wide), start);
                    }
                    n
                } else {
                    u8::from_be_bytes([b & 0b000_11111]) as u64
                };

                return Ok(len as usize);
            }
            _ => return self.p.fail_at_position(expected, self.p.position() - 1),
        }
    }
}

impl<'a, 'de> de::Deserializer<'de> for &'a mut VVDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.p.peek()? & 0b111_00000 {
            0b000_00000 => {
                self.parse_nil()?;
                visitor.visit_unit()
            }
            0b001_00000 => self.deserialize_bool(visitor),
            0b010_00000 => self.deserialize_f64(visitor),
            0b011_00000 => self.deserialize_i64(visitor),
            0b100_00000 => self.deserialize_bytes(visitor),
            0b101_00000 => self.deserialize_seq(visitor),
            0b110_00000 => self.deserialize_map(visitor),
            0b111_00000 => self.deserialize_map(visitor),
            _ => unreachable!(),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
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
        visitor.visit_f64(self.parse_float()?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.parse_float()?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let start = self.p.position();
        let n = self.parse_int()?;
        if n < 0 || n > std::u32::MAX as i64 {
            return self.p.fail_at_position(DecodeError::OutOfBoundsChar, start);
        } else {
            match char::from_u32(n as u32) {
                Some(c) => return visitor.visit_char(c),
                None => return self.p.fail_at_position(DecodeError::OutOfBoundsChar, start),
            }
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let bytes = self.parse_bytes()?;
        match std::str::from_utf8(bytes) {
            Ok(s) => visitor.visit_str(s),
            Err(_) => self.p.fail(DecodeError::Utf8),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if (self.p.peek()? & 0b111_00000) == 0b101_00000 {
            let v = Vec::deserialize(&mut *self)?;
            match String::from_utf8(v) {
                Ok(s) => visitor.visit_string(s),
                Err(_) => self.p.fail(DecodeError::Utf8),
            }
        } else {
            let bytes = self.parse_bytes()?;
            match std::str::from_utf8(bytes) {
                Ok(s) => visitor.visit_string(s.to_string()),
                Err(_) => self.p.fail(DecodeError::Utf8),
            }
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bytes(self.parse_bytes()?)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if (self.p.peek()? & 0b111_00000) == 0b101_00000 {
            let v = Vec::deserialize(self)?;
            return visitor.visit_byte_buf(v);
        } else {
            let bytes = self.parse_bytes()?;
            return visitor.visit_byte_buf(bytes.to_owned());
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.p.advance_over(&[0b100_00100, 'N' as u8, 'o' as u8, 'n' as u8, 'e' as u8]) {
            return visitor.visit_none();
        } else if self.p.advance_over(&[0b111_00001, 0b100_00100, 'S' as u8, 'o' as u8, 'm' as u8, 'e' as u8]) {
            return visitor.visit_some(self);
        } else {
            return self.p.fail(DecodeError::ExpectedOption);
        }
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
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let count = self.parse_count(0b101_00000, DecodeError::ExpectedArray, CanonicityCondition::ArrayTooWide, DecodeError::OutOfBoundsArray)?;
        return visitor.visit_seq(SequenceAccessor::new(&mut self, count));
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
        match self.p.peek()? & 0b111_00000 {
            0b110_00000 => {
                if self.canonic {
                    return self.p.fail(DecodeError::Canonicity(CanonicityCondition::Set));
                }
                let count = self.parse_count(0b110_00000, DecodeError::ExpectedMap, CanonicityCondition::MapTooWide, DecodeError::OutOfBoundsSet)?;
                return visitor.visit_map(MapAccessor::new(&mut self, count, true));
            }
            0b111_00000 => {
                let count = self.parse_count(0b110_00000, DecodeError::ExpectedMap, CanonicityCondition::MapTooWide, DecodeError::OutOfBoundsMap)?;
                return visitor.visit_map(MapAccessor::new(&mut self, count, false));
            }
            _ => return self.p.fail(DecodeError::ExpectedMap),
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
        match self.p.peek()? & 0b111_00000 {
            0b100_00000 | 0b110_00000 | 0b111_00000 => Ok(visitor.visit_enum(Enum::new(self))?),
            _ => self.p.fail(DecodeError::ExpectedEnum(name.to_string()))
        }
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
        false
    }
}

struct SequenceAccessor<'a, 'de> {
    des: &'a mut VVDeserializer<'de>,
    len: usize,
    read: usize,
}

impl<'a, 'de> SequenceAccessor<'a, 'de> {
    fn new(des: &'a mut VVDeserializer<'de>, len: usize) -> SequenceAccessor<'a, 'de> {
        SequenceAccessor { des, len, read: 0 }
    }
}

impl<'a, 'de> SeqAccess<'de> for SequenceAccessor<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.read < self.len {
            let inner = seed.deserialize(&mut *self.des)?;
            self.read += 1;
            return Ok(Some(inner));
        } else {
            return Ok(None);
        }
    }
}

struct MapAccessor<'a, 'de> {
    des: &'a mut VVDeserializer<'de>,
    len: usize,
    read: usize,
    set: bool,
}

impl<'a, 'de> MapAccessor<'a, 'de> {
    fn new(des: &'a mut VVDeserializer<'de>, len: usize, set: bool) -> MapAccessor<'a, 'de> {
        MapAccessor { des, len, read: 0, set }
    }
}

impl<'a, 'de> MapAccess<'de> for MapAccessor<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.read < self.len {
            let inner = seed.deserialize(&mut *self.des)?;
            return Ok(Some(inner));
        } else {
            return Ok(None);
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let value = if self.set {
            match seed.deserialize(AlwaysNil::new()) {
                Ok(nil) => nil,
                Err(_) => return self.des.p.fail(DecodeError::InvalidSet),
            }
        } else {
            seed.deserialize(&mut *self.des)?
        };
        self.read += 1;
        return Ok(value);
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
            b if b & 0b111_00000 == 0b100_00000 => Ok((seed.deserialize(&mut *self.des)?, self)),
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
