use core::marker::PhantomData;
use std::fmt;

use thiserror::Error;
use serde::de::{
    self, DeserializeSeed, SeqAccess, Visitor,
};

use atm_parser_helper::Error;

pub struct BytesAsSeq<E> {
    b: Vec<u8>,
    i: usize,
    err_position: usize,
    e_bounds: E,
    e_int: E,
}

impl<E: Clone> BytesAsSeq<E> {
    pub fn new(b: Vec<u8>, err_position: usize, e_bounds: E, e_int: E) -> Self {
        BytesAsSeq { b, i: 0, err_position, e_bounds, e_int }
    }
}

impl<'de, E: Clone + serde::de::Error> SeqAccess<'de> for BytesAsSeq<E> {
    type Error = Error<E>;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.b.get(self.i) {
            Some(b) => {
                self.i += 1;
                return seed.deserialize(IntFromByte {
                    b: *b,
                    err_position: self.err_position,
                    e_bounds: self.e_bounds.clone(),
                    e_int: self.e_int.clone(),
                }).map(|yay| Some(yay));
            }
            None => Ok(None),
        }
    }
}

struct IntFromByte<E> {
    b: u8,
    err_position: usize,
    e_bounds: E,
    e_int: E,
}

impl<'de, E: serde::de::Error + Clone> de::Deserializer<'de> for IntFromByte<E> {
    type Error = Error<E>;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.b <= (i8::MAX as u8) {
            visitor.visit_i8(self.b as i8)
        } else {
            Err(Error::new(self.err_position, self.e_bounds.clone()))
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.b.into())
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.b.into())
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.b.into())
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.b.into())
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.b.into())
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.b.into())
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.b.into())
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_char(self.b.into())
    }

    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_string<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::new(self.err_position, self.e_int.clone()))
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

#[derive(Error, Debug)]
#[error("can only decode a set where a map that maps all keys to nil would be valid")]
pub struct AlwaysNilError;

impl de::Error for AlwaysNilError {
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        AlwaysNilError
    }
}

pub struct AlwaysNil<'de>(PhantomData<&'de ()>);

impl<'de> AlwaysNil<'de> {
    pub fn new() -> Self {
        AlwaysNil(PhantomData)
    }
}

impl<'a, 'de> de::Deserializer<'de> for AlwaysNil<'de> {
    type Error = AlwaysNilError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_i32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_string<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
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

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(AlwaysNilError)
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
