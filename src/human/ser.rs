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

/// A structure that serializes valuable values in the human-readable encoding.
///
/// https://github.com/AljoschaMeyer/valuable-value/blob/main/README.md
pub struct VVSerializer {
    out: Vec<u8>,
    indentation: usize,
    current_indentation: usize,
    multiline: bool,
}

impl VVSerializer {
    /// Create a new serializer, writing human-readable encoding into the given Vec.
    /// Does pretty-printing if the indentation is greater than zero.
    pub fn new(out: Vec<u8>, indentation: usize) -> Self {
        VVSerializer { out, indentation, current_indentation: 0, multiline: false }
    }
}

/// Write human-readable encoding into a Vec.
/// Does pretty-printing if the indentation is greater than zero.
pub fn to_vec<T>(value: &T, indentation: usize) -> Result<Vec<u8>, EncodeError>
where
    T: Serialize,
{
    let mut serializer = VVSerializer::new(Vec::new(), indentation);
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
        Ok(self.out.extend_from_slice(if v { b"true" } else { b"false" }))
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
        let mut buffer = itoa::Buffer::new();
        self.out.extend_from_slice(buffer.format(v).as_bytes());
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
        if v.is_nan() {
            self.out.extend_from_slice(b"NaN");
        } else if v == f64::INFINITY {
            self.out.extend_from_slice(b"Inf");
        } else if v == f64::NEG_INFINITY {
            self.out.extend_from_slice(b"-Inf");
        } else {
            let config = pretty_dtoa::FmtFloatConfig::default()
                .add_point_zero(true);
            self.out.extend_from_slice(pretty_dtoa::dtoa(v, config).as_bytes());
        }

        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<(), EncodeError> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<(), EncodeError> {
        self.out.extend_from_slice(format!("{}", v).as_bytes());
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<(), EncodeError> {
        self.out.extend_from_slice(b"@[");

        match v.len() {
            0 => self.out.push(']' as u8),
            1 => {
                self.serialize_u8(v[0])?;
                self.out.push(']' as u8);
            }
            _ if self.indentation == 0 => {
                for i in v.iter() {
                    self.serialize_u8(*i)?;
                    self.out.extend_from_slice(b", ");
                }
                self.out.push(']' as u8);
            }
            _ => {
                self.out.push('\n' as u8);
                self.current_indentation += 1;

                for i in v.iter() {
                    for _ in 0..self.current_indentation {
                        for _ in 0..self.indentation {
                            self.out.push(' ' as u8);
                        }
                    }
                    self.serialize_u8(*i)?;
                    self.out.extend_from_slice(b",\n");
                }

                self.current_indentation -= 1;
                for _ in 0..self.current_indentation {
                    for _ in 0..self.indentation {
                        self.out.push(' ' as u8);
                    }
                }
                self.out.push(']' as u8);
            }
        }

        return Ok(());
    }

    fn serialize_none(self) -> Result<(), EncodeError> {
        self.serialize_str("None")
    }

    fn serialize_some<T>(self, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        self.out.extend_from_slice(b"{\"Some\":");
        if self.indentation != 0 {
            self.out.push(' ' as u8);
        }
        value.serialize(&mut *self)?;
        self.out.push('}' as u8);
        Ok(())
    }

    fn serialize_unit(self) -> Result<(), EncodeError> {
        Ok(self.out.extend_from_slice(b"nil"))
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
        self.out.push('{' as u8);
        variant.serialize(&mut *self)?;
        self.out.extend_from_slice(b": ");
        value.serialize(&mut *self)?;
        self.out.push('}' as u8);
        Ok(())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.out.push('[' as u8);
        match len {
            Some(0 | 1) => self.multiline = false,
            _ => {
                self.multiline = true;
                self.current_indentation += 1;
            }
        }
        Ok(self)
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
        unimplemented!();
        // self.out.push(0b111_00001);
        // variant.serialize(&mut *self)?;
        // if len != 1 {
        //     self.serialize_count(len, 0b101_00000)?;
        // }
        // Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.out.push('{' as u8);
        match len {
            Some(0 | 1) => self.multiline = false,
            _ => {
                self.multiline = true;
                self.current_indentation += 1;
            }
        }
        Ok(self)
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
        if self.multiline {
            for _ in 0..self.current_indentation {
                for _ in 0..self.indentation {
                    self.out.push(' ' as u8);
                }
            }
        }
        let old = self.multiline;
        value.serialize(&mut **self)?;
        self.multiline = old;

        if self.multiline {
            self.out.push(',' as u8);
            if self.indentation != 0 {
                self.out.push('\n' as u8);
            }
        }

        Ok(())
    }

    fn end(self) -> Result<(), EncodeError> {
        if self.multiline {
            self.current_indentation -= 1;
            for _ in 0..self.current_indentation {
                for _ in 0..self.indentation {
                    self.out.push(' ' as u8);
                }
            }
        }
        self.out.push(']' as u8);
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
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<(), EncodeError> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut VVSerializer {
    type Ok = ();
    type Error = EncodeError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<(), EncodeError> {
        ser::SerializeSeq::end(self)
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
        if self.multiline {
            for _ in 0..self.current_indentation {
                for _ in 0..self.indentation {
                    self.out.push(' ' as u8);
                }
            }
        }
        let old = self.multiline;
        key.serialize(&mut **self)?;
        self.multiline = old;

        self.out.push(':' as u8);
        if self.indentation != 0 {
            self.out.push(' ' as u8);
        }

        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        let old = self.multiline;
        value.serialize(&mut **self)?;
        self.multiline = old;

        if self.multiline {
            self.out.push(',' as u8);
            if self.indentation != 0 {
                self.out.push('\n' as u8);
            }
        }
        Ok(())
    }

    fn end(self) -> Result<(), EncodeError> {
        if self.multiline {
            self.current_indentation -= 1;
            for _ in 0..self.current_indentation {
                for _ in 0..self.indentation {
                    self.out.push(' ' as u8);
                }
            }
        }
        self.out.push('}' as u8);
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
