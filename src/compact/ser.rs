use std::fmt;

use serde::ser::{self, Serializer, Serialize};
use thiserror::Error;

/// Everything that can go wrong during serialization.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum EncodeError {
    #[error("{0}")]
    Message(String),
    #[error("valuable value ints cannot exceed 2^63 - 1")]
    OutOfBoundsInt,
    #[error("collection length cannot exceed 2^63 - 1")]
    OutOfBoundsCollection,
    #[error("collections must have a known length")]
    UnknownLength,
}

impl serde::ser::Error for EncodeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        EncodeError::Message(msg.to_string())
    }
}

/// A structure that serializes valuable values in the compact encoding.
///
/// https://github.com/AljoschaMeyer/valuable-value/blob/main/README.md
pub struct VVSerializer {
    out: Vec<u8>,
}

impl VVSerializer {
    fn serialize_count(&mut self, n: usize, tag: u8) -> Result<(), EncodeError> {
        if n <= 27 {
            self.out.push(tag | (n as u8));
        } else if n <= (u8::MAX as usize) {
            self.out.push(tag | 0b000_11100);
            self.out.extend_from_slice(&(n as u8).to_be_bytes());
        } else if n <= (u16::MAX as usize) {
            self.out.push(tag | 0b000_11101);
            self.out.extend_from_slice(&(n as u16).to_be_bytes());
        } else if n <= (u32::MAX as usize) {
            self.out.push(tag | 0b000_11101);
            self.out.extend_from_slice(&(n as u32).to_be_bytes());
        } else if n <= (i64::MAX as usize) {
            self.out.push(tag | 0b000_11111);
            self.out.extend_from_slice(&(n as u64).to_be_bytes());
        } else {
            return Err(EncodeError::OutOfBoundsCollection)
        }

        Ok(())
    }
}

pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, EncodeError>
where
    T: Serialize,
{
    let mut serializer = VVSerializer {
        out: Vec::new(),
    };
    value.serialize(&mut serializer)?;
    Ok(serializer.out)
}

impl<'a> Serializer for &'a mut VVSerializer {
    type Ok = ();
    type Error = EncodeError;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<(), EncodeError> {
        Ok(self.out.push(if v { 0b001_00001 } else { 0b001_00000 }))
    }

    fn serialize_i8(self, v: i8) -> Result<(), EncodeError> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<(), EncodeError> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<(), EncodeError> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<(), EncodeError> {
        if 0 <= v && v <= 27 {
            self.out.push(0b011_00000 | (v as u8));
        } else if (i8::MIN as i64) <= v && v <= (i8::MAX as i64) {
            self.out.push(0b011_11100);
            self.out.extend_from_slice(&(v as i8).to_be_bytes());
        } else if (i16::MIN as i64) <= v && v <= (i16::MAX as i64) {
            self.out.push(0b011_11101);
            self.out.extend_from_slice(&(v as i16).to_be_bytes());
        } else if (i32::MIN as i64) <= v && v <= (i32::MAX as i64) {
            self.out.push(0b011_11110);
            self.out.extend_from_slice(&(v as i32).to_be_bytes());
        } else {
            self.out.push(0b011_11111);
            self.out.extend_from_slice(&(v as i64).to_be_bytes());
        }

        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<(), EncodeError> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<(), EncodeError> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<(), EncodeError> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<(), EncodeError> {
        if v <= (i64::MAX as u64) {
            self.serialize_i64(v as i64)
        } else {
            Err(EncodeError::OutOfBoundsInt)
        }
    }

    fn serialize_f32(self, v: f32) -> Result<(), EncodeError> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<(), EncodeError> {
        self.out.push(0b010_00000);
        self.out.extend_from_slice(&v.to_bits().to_be_bytes());
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<(), EncodeError> {
        self.serialize_u32(v as u32)
    }

    fn serialize_str(self, v: &str) -> Result<(), EncodeError> {
        self.serialize_bytes(v.as_bytes())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<(), EncodeError> {
        self.serialize_count(v.len(), 0b100_00000)?;
        self.out.extend_from_slice(v);
        return Ok(());
    }

    fn serialize_none(self) -> Result<(), EncodeError> {
        self.serialize_str("None")
    }

    fn serialize_some<T>(self, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        self.out.push(0b111_00001);
        self.serialize_str("Some")?;
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<(), EncodeError> {
        Ok(self.out.push(0b000_00000))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), EncodeError> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<(), EncodeError> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        self.out.push(0b111_00001);
        variant.serialize(&mut *self)?;
        value.serialize(&mut *self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        match len {
            None => return Err(EncodeError::UnknownLength),
            Some(len) => {
                self.serialize_count(len, 0b101_00000)?;
                return Ok(self);
            }
        }
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.out.push(0b111_00001);
        variant.serialize(&mut *self)?;
        if len != 1 {
            self.serialize_count(len, 0b101_00000)?;
        }
        Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        match len {
            None => return Err(EncodeError::UnknownLength),
            Some(len) => {
                self.serialize_count(len, 0b111_00000)?;
                return Ok(self);
            }
        }
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.out.push(0b111_00001);
        variant.serialize(&mut *self)?;
        Ok(self)
    }
}

impl<'a> ser::SerializeSeq for &'a mut VVSerializer {
    type Ok = ();
    type Error = EncodeError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError> {
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut VVSerializer {
    type Ok = ();
    type Error = EncodeError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut VVSerializer {
    type Ok = ();
    type Error = EncodeError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut VVSerializer {
    type Ok = ();
    type Error = EncodeError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError> {
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &'a mut VVSerializer {
    type Ok = ();
    type Error = EncodeError;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError> {
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for &'a mut VVSerializer {
    type Ok = ();
    type Error = EncodeError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut **self)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError> {
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut VVSerializer {
    type Ok = ();
    type Error = EncodeError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        self.out.push(0b111_00001);
        key.serialize(&mut **self)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError> {
        Ok(())
    }
}
