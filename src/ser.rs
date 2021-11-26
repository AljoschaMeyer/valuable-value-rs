use serde::ser::Error;
use std::fmt;

use serde::ser::{self, Serializer, Serialize};
use thiserror::Error;
use pretty_dtoa::{dtoa, FmtFloatConfig};

/// Everything that can go wrong during serialization.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
#[error("{msg}")]
pub struct EncodeError {
    msg: String,
}

impl serde::ser::Error for EncodeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        EncodeError {msg: msg.to_string() }
    }
}

/// Describes how serialized values should be encoded.
pub enum Format {
    /// Produce output in the canonic encoding.
    Canonic,
    /// Produce human-readable output with the given number of spaces used for indentation.
    HumanReadable(usize),
}

/// A structure that serializes valuable values.
///
/// https://github.com/AljoschaMeyer/valuable-value/blob/main/README.md
pub struct VVSerializer {
    out: Vec<u8>,
    format: Format,
}

pub fn to_vec<T>(value: &T, f: Format) -> Result<Vec<u8>, EncodeError>
where
    T: Serialize,
{
    let mut serializer = VVSerializer {
        out: Vec::new(),
        format: f,
    };
    value.serialize(&mut serializer)?;
    Ok(serializer.out)
}

pub fn to_string<T>(value: &T, indentation: usize) -> Result<String, EncodeError>
where
    T: Serialize,
{
    let mut serializer = VVSerializer {
        out: Vec::new(),
        format: Format::HumanReadable(indentation),
    };
    value.serialize(&mut serializer)?;
    Ok(unsafe { String::from_utf8_unchecked(serializer.out) })
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
        match self.format {
            Format::Canonic => Ok(self.out.push(if v { 0b1_010_1110 } else { 0b1_010_1101 })),
            Format::HumanReadable(_) => Ok(self.out.extend_from_slice(if v { b"true" } else { b"false" })),
        }
    }

    // JSON does not distinguish between different sizes of integers, so all
    // signed integers will be serialized the same and all unsigned integers
    // will be serialized the same. Other formats, especially compact binary
    // formats, may need independent logic for the different sizes.
    fn serialize_i8(self, v: i8) -> Result<(), EncodeError> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<(), EncodeError> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<(), EncodeError> {
        self.serialize_i64(i64::from(v))
    }

    // Not particularly efficient but this is example code anyway. A more
    // performant approach would be to use the `itoa` crate.
    fn serialize_i64(self, v: i64) -> Result<(), EncodeError> {
        match self.format {
            Format::Canonic => {
                if 0 <= v && v <= 11 {
                    self.out.push(0b1_011_0000 ^ (v as u8));
                } else if -128 <= v && v <= 127 {
                    self.out.push(0b1_011_1100);
                    self.out.extend_from_slice(&(v as i8).to_be_bytes());
                } else if -32768 <= v && v <= 32767 {
                    self.out.push(0b1_011_1101);
                    self.out.extend_from_slice(&(v as i16).to_be_bytes());
                } else if -2147483648 <= v && v <= 2147483647 {
                    self.out.push(0b1_011_1110);
                    self.out.extend_from_slice(&(v as i32).to_be_bytes());
                } else {
                    self.out.push(0b1_011_1111);
                    self.out.extend_from_slice(&(v as i64).to_be_bytes());
                };

                Ok(())
            }
            Format::HumanReadable(_) => {
                self.out.extend_from_slice(format!("{}", v).as_bytes());
                Ok(())
            }
        }
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
            Err(EncodeError::custom(format!("integer is not a i64: {}", v)))
        }
    }

    fn serialize_f32(self, v: f32) -> Result<(), EncodeError> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<(), EncodeError> {
        match self.format {
            Format::HumanReadable(_) => {
                if v.is_nan() {
                    self.out.extend_from_slice(b"NaN");
                    Ok(())
                } else if v == f64::INFINITY {
                    self.out.extend_from_slice(b"Inf");
                    Ok(())
                } else if v == f64::NEG_INFINITY {
                    self.out.extend_from_slice(b"-Inf");
                    Ok(())
                } else {
                    let config = FmtFloatConfig::default().add_point_zero(true);
                    self.out.extend_from_slice(dtoa(v, config).as_bytes());
                    Ok(())
                }
            }
            Format::Canonic => {
                self.out.push(0b1_010_1111);
                self.out.extend_from_slice(&v.to_bits().to_be_bytes());
                Ok(())
            }
        }
    }

    // Serialize a char as a single-character string. Other formats may
    // represent this differently.
    fn serialize_char(self, v: char) -> Result<(), EncodeError> {
        unimplemented!()
        // self.serialize_str(&v.to_string())
    }

    // This only works for strings that don't require escape sequences but you
    // get the idea. For example it would emit invalid JSON if the input string
    // contains a '"' character.
    fn serialize_str(self, v: &str) -> Result<(), EncodeError> {
        unimplemented!()
        // self.output += "\"";
        // self.output += v;
        // self.output += "\"";
        // Ok(())
    }

    // Serialize a byte array as an array of bytes. Could also use a base64
    // string here. Binary formats will typically represent byte arrays more
    // compactly.
    fn serialize_bytes(self, v: &[u8]) -> Result<(), EncodeError> {
        unimplemented!()
        // use serde::ser::SerializeSeq;
        // let mut seq = self.serialize_seq(Some(v.len()))?;
        // for byte in v {
        //     seq.serialize_element(byte)?;
        // }
        // seq.end()
    }

    // An absent optional is represented as the JSON `null`.
    fn serialize_none(self) -> Result<(), EncodeError> {
        unimplemented!()
        // self.serialize_unit()
    }

    // A present optional is represented as just the contained value. Note that
    // this is a lossy representation. For example the values `Some(())` and
    // `None` both serialize as just `null`. Unfortunately this is typically
    // what people expect when working with JSON. Other formats are encouraged
    // to behave more intelligently if possible.
    fn serialize_some<T>(self, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!()
        // value.serialize(self)
    }

    fn serialize_unit(self) -> Result<(), EncodeError> {
        match self.format {
            Format::Canonic => Ok(self.out.push(0b1_010_1100)),
            Format::HumanReadable(_) => Ok(self.out.extend_from_slice(b"nil")),
        }
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), EncodeError> {
        self.serialize_unit()
    }

    // When serializing a unit variant (or any other kind of variant), formats
    // can choose whether to keep track of it by index or by name. Binary
    // formats typically use the index of the variant and human-readable formats
    // typically use the name.
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<(), EncodeError> {
        unimplemented!()
        // self.serialize_str(variant)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain.
    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!()
        // value.serialize(self)
    }

    // Note that newtype variant (and all of the other variant serialization
    // methods) refer exclusively to the "externally tagged" enum
    // representation.
    //
    // Serialize this to JSON in externally tagged form as `{ NAME: VALUE }`.
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
        unimplemented!()
        // self.output += "{";
        // variant.serialize(&mut *self)?;
        // self.output += ":";
        // value.serialize(&mut *self)?;
        // self.output += "}";
        // Ok(())
    }

    // Now we get to the serialization of compound types.
    //
    // The start of the sequence, each value, and the end are three separate
    // method calls. This one is responsible only for serializing the start,
    // which in JSON is `[`.
    //
    // The length of the sequence may or may not be known ahead of time. This
    // doesn't make a difference in JSON because the length is not represented
    // explicitly in the serialized form. Some serializers may only be able to
    // support sequences for which the length is known up front.
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        match self.format {
            Format::Canonic => {
                match len {
                    None => return Err(EncodeError::custom("cannot serialize a sequence of unknown length")),
                    Some(len) => {
                        if len <= 11 {
                            self.out.push(0b1_101_0000 ^ (len as u8));
                        } else if len <= u8::MAX as usize {
                            self.out.push(0b1_101_1100);
                            self.out.extend_from_slice(&(len as u8).to_be_bytes());
                        } else if len <= u16::MAX as usize {
                            self.out.push(0b1_101_1101);
                            self.out.extend_from_slice(&(len as u16).to_be_bytes());
                        } else if len <= u32::MAX as usize {
                            self.out.push(0b1_101_1110);
                            self.out.extend_from_slice(&(len as u32).to_be_bytes());
                        } else {
                            self.out.push(0b1_101_1111);
                            self.out.extend_from_slice(&(len as u64).to_be_bytes());
                        };

                        return Ok(self);
                    }
                }
            }
            Format::HumanReadable(_) => {
                self.out.push('[' as u8);
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
        unimplemented!()
        // self.serialize_seq(Some(len))
    }

    // Tuple variants are represented in JSON as `{ NAME: [DATA...] }`. Again
    // this method is only responsible for the externally tagged representation.
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        unimplemented!()
        // self.output += "{";
        // variant.serialize(&mut *self)?;
        // self.output += ":[";
        // Ok(self)
    }

    // Maps are represented in JSON as `{ K: V, K: V, ... }`.
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        unimplemented!()
        // self.output += "{";
        // Ok(self)
    }

    // Structs look just like maps in JSON. In particular, JSON requires that we
    // serialize the field names of the struct. Other formats may be able to
    // omit the field names when serializing structs because the corresponding
    // Deserialize implementation is required to know what the keys are without
    // looking at the serialized data.
    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        unimplemented!()
        // self.serialize_map(Some(len))
    }

    // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }`.
    // This is the externally tagged representation.
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        unimplemented!()
        // self.output += "{";
        // variant.serialize(&mut *self)?;
        // self.output += ":{";
        // Ok(self)
    }
}

impl<'a> ser::SerializeSeq for &'a mut VVSerializer {
    type Ok = ();
    type Error = EncodeError;

    // Serialize a single element of the sequence.
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        match self.format {
            Format::Canonic => value.serialize(&mut **self),
            Format::HumanReadable(_) => {
                // println!("{:?}", self.out);
                if !(self.out.last().unwrap() == &('[' as u8)) {
                    self.out.extend_from_slice(b", ");
                }
                value.serialize(&mut **self)
            }
        }
    }

    // Close the sequence.
    fn end(self) -> Result<(), EncodeError> {
        match self.format {
            Format::Canonic => Ok(()),
            Format::HumanReadable(_) => {
                self.out.push(']' as u8);
                Ok(())
            }
        }
    }
}

// Same thing but for tuples.
impl<'a> ser::SerializeTuple for &'a mut VVSerializer

{
    type Ok = ();
    type Error = EncodeError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!();
        // if !self.output.ends_with('[') {
        //     self.output += ",";
        // }
        // value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError> {
        unimplemented!();
        // self.output += "]";
        // Ok(())
    }
}

// Same thing but for tuple structs.
impl<'a> ser::SerializeTupleStruct for &'a mut VVSerializer

{
    type Ok = ();
    type Error = EncodeError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!();
        // if !self.output.ends_with('[') {
        //     self.output += ",";
        // }
        // value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError> {
        unimplemented!();
        // self.output += "]";
        // Ok(())
    }
}

// Tuple variants are a little different. Refer back to the
// `serialize_tuple_variant` method above:
//
//    self.output += "{";
//    variant.serialize(&mut *self)?;
//    self.output += ":[";
//
// So the `end` method in this impl is responsible for closing both the `]` and
// the `}`.
impl<'a> ser::SerializeTupleVariant for &'a mut VVSerializer

{
    type Ok = ();
    type Error = EncodeError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!();
        // if !self.output.ends_with('[') {
        //     self.output += ",";
        // }
        // value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError> {
        unimplemented!();
        // self.output += "]}";
        // Ok(())
    }
}

// Some `Serialize` types are not able to hold a key and value in memory at the
// same time so `SerializeMap` implementations are required to support
// `serialize_key` and `serialize_value` individually.
//
// There is a third optional method on the `SerializeMap` trait. The
// `serialize_entry` method allows serializers to optimize for the case where
// key and value are both available simultaneously. In JSON it doesn't make a
// difference so the default behavior for `serialize_entry` is fine.
impl<'a> ser::SerializeMap for &'a mut VVSerializer

{
    type Ok = ();
    type Error = EncodeError;

    // The Serde data model allows map keys to be any serializable type. JSON
    // only allows string keys so the implementation below will produce invalid
    // JSON if the key serializes as something other than a string.
    //
    // A real JSON serializer would need to validate that map keys are strings.
    // This can be done by using a different Serializer to serialize the key
    // (instead of `&mut **self`) and having that other serializer only
    // implement `serialize_str` and return an error on any other data type.
    fn serialize_key<T>(&mut self, key: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!();
        // if !self.output.ends_with('{') {
        //     self.output += ",";
        // }
        // key.serialize(&mut **self)
    }

    // It doesn't make a difference whether the colon is printed at the end of
    // `serialize_key` or at the beginning of `serialize_value`. In this case
    // the code is a bit simpler having it here.
    fn serialize_value<T>(&mut self, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!();
        // self.output += ":";
        // value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError> {
        unimplemented!();
        // self.output += "}";
        // Ok(())
    }
}

// Structs are like maps in which the keys are constrained to be compile-time
// constant strings.
impl<'a> ser::SerializeStruct for &'a mut VVSerializer

{
    type Ok = ();
    type Error = EncodeError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!();
        // if !self.output.ends_with('{') {
        //     self.output += ",";
        // }
        // key.serialize(&mut **self)?;
        // self.output += ":";
        // value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError> {
        unimplemented!();
        // self.output += "}";
        // Ok(())
    }
}

// Similar to `SerializeTupleVariant`, here the `end` method is responsible for
// closing both of the curly braces opened by `serialize_struct_variant`.
impl<'a> ser::SerializeStructVariant for &'a mut VVSerializer

{
    type Ok = ();
    type Error = EncodeError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), EncodeError>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!();
        // if !self.output.ends_with('{') {
        //     self.output += ",";
        // }
        // key.serialize(&mut **self)?;
        // self.output += ":";
        // value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError> {
        unimplemented!();
        // self.output += "}}";
        // Ok(())
    }
}
