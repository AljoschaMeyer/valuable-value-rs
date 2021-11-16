use std::fmt;

use panda_pile::sync::{BulkConsumer};
use serde::ser::{self, Serializer, Serialize};
use thiserror::Error;

/// Everything that can go wrong during deserialization.
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum EncodeError<E> {
    /// Custom, stringly-typed error.
    #[error("{0}")]
    Message(String),
    /// An error on the [`BulkConsumer`](panda_pile::sync::BulkConsumer) into which the Serializer writes data.
    #[error("{0}")]
    Consumer(E),

}

impl<E: fmt::Display + fmt::Debug> serde::ser::Error for EncodeError<E> {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        EncodeError::Message(msg.to_string())
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
pub struct VVSerializer<'s, C> {
    consumer: &'s mut C,
    format: Format,
}

pub fn to_bulk_consumer<T, C, E>(value: &T, consumer: &mut C, f: Format) -> Result<(), EncodeError<E>>
where
    T: Serialize,
    E: fmt::Display + fmt::Debug,
    C: BulkConsumer<Repeated = u8, Stopped = E>,
{
    let mut serializer = VVSerializer {
        consumer,
        format: f,
    };
    value.serialize(&mut serializer)?;
    Ok(())
}

impl<'a, 's, C, E> Serializer for &'a mut VVSerializer<'s, C>
where
    E: fmt::Display + fmt::Debug,
    C: BulkConsumer<Repeated = u8, Stopped = E>,
{
    type Ok = ();
    type Error = EncodeError<E>;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<(), EncodeError<E>> {
        unimplemented!()
        // self.output += if v { "true" } else { "false" };
        // Ok(())
    }

    // JSON does not distinguish between different sizes of integers, so all
    // signed integers will be serialized the same and all unsigned integers
    // will be serialized the same. Other formats, especially compact binary
    // formats, may need independent logic for the different sizes.
    fn serialize_i8(self, v: i8) -> Result<(), EncodeError<E>> {
        unimplemented!()
        // self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<(), EncodeError<E>> {
        unimplemented!()
        // self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<(), EncodeError<E>> {
        unimplemented!()
        // self.serialize_i64(i64::from(v))
    }

    // Not particularly efficient but this is example code anyway. A more
    // performant approach would be to use the `itoa` crate.
    fn serialize_i64(self, v: i64) -> Result<(), EncodeError<E>> {
        unimplemented!()
        // self.output += &v.to_string();
        // Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<(), EncodeError<E>> {
        unimplemented!()
        // self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<(), EncodeError<E>> {
        unimplemented!()
        // self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<(), EncodeError<E>> {
        unimplemented!()
        // self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<(), EncodeError<E>> {
        unimplemented!()
        // self.output += &v.to_string();
        // Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<(), EncodeError<E>> {
        unimplemented!()
        // self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<(), EncodeError<E>> {
        unimplemented!()
        // self.output += &v.to_string();
        // Ok(())
    }

    // Serialize a char as a single-character string. Other formats may
    // represent this differently.
    fn serialize_char(self, v: char) -> Result<(), EncodeError<E>> {
        unimplemented!()
        // self.serialize_str(&v.to_string())
    }

    // This only works for strings that don't require escape sequences but you
    // get the idea. For example it would emit invalid JSON if the input string
    // contains a '"' character.
    fn serialize_str(self, v: &str) -> Result<(), EncodeError<E>> {
        unimplemented!()
        // self.output += "\"";
        // self.output += v;
        // self.output += "\"";
        // Ok(())
    }

    // Serialize a byte array as an array of bytes. Could also use a base64
    // string here. Binary formats will typically represent byte arrays more
    // compactly.
    fn serialize_bytes(self, v: &[u8]) -> Result<(), EncodeError<E>> {
        unimplemented!()
        // use serde::ser::SerializeSeq;
        // let mut seq = self.serialize_seq(Some(v.len()))?;
        // for byte in v {
        //     seq.serialize_element(byte)?;
        // }
        // seq.end()
    }

    // An absent optional is represented as the JSON `null`.
    fn serialize_none(self) -> Result<(), EncodeError<E>> {
        unimplemented!()
        // self.serialize_unit()
    }

    // A present optional is represented as just the contained value. Note that
    // this is a lossy representation. For example the values `Some(())` and
    // `None` both serialize as just `null`. Unfortunately this is typically
    // what people expect when working with JSON. Other formats are encouraged
    // to behave more intelligently if possible.
    fn serialize_some<T>(self, value: &T) -> Result<(), EncodeError<E>>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!()
        // value.serialize(self)
    }

    // In Serde, unit means an anonymous value containing no data. Map this to
    // JSON as `null`.
    fn serialize_unit(self) -> Result<(), EncodeError<E>> {
        match self.format {
            Format::Canonic => unimplemented!(), // 0b1_010_1100
            Format::HumanReadable(_) => unimplemented!(),
        }
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), EncodeError<E>> {
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
    ) -> Result<(), EncodeError<E>> {
        unimplemented!()
        // self.serialize_str(variant)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain.
    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<(), EncodeError<E>>
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
    ) -> Result<(), EncodeError<E>>
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
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        unimplemented!()
        // self.output += "[";
        // Ok(self)
    }

    // Tuples look just like sequences in JSON. Some formats may be able to
    // represent tuples more efficiently by omitting the length, since tuple
    // means that the corresponding `Deserialize implementation will know the
    // length without needing to look at the serialized data.
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        unimplemented!()
        // self.serialize_seq(Some(len))
    }

    // Tuple structs look just like sequences in JSON.
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

// The following 7 impls deal with the serialization of compound types like
// sequences and maps. Serialization of such types is begun by a Serializer
// method and followed by zero or more calls to serialize individual elements of
// the compound type and one call to end the compound type.
//
// This impl is SerializeSeq so these methods are called after `serialize_seq`
// is called on the Serializer.
impl<'a, 's, C, E> ser::SerializeSeq for &'a mut VVSerializer<'s, C>
where
    E: fmt::Display + fmt::Debug,
    C: BulkConsumer<Repeated = u8, Stopped = E>,
{
    // Must match the `Ok` type of the serializer.
    type Ok = ();
    // Must match the `Error` type of the serializer.
    type Error = EncodeError<E>;

    // Serialize a single element of the sequence.
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), EncodeError<E>>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!();
        // if !self.output.ends_with('[') {
        //     self.output += ",";
        // }
        // value.serialize(&mut **self)
    }

    // Close the sequence.
    fn end(self) -> Result<(), EncodeError<E>> {
        unimplemented!();
        // self.output += "]";
        // Ok(())
    }
}

// Same thing but for tuples.
impl<'a, 's, C, E> ser::SerializeTuple for &'a mut VVSerializer<'s, C>
where
    E: fmt::Display + fmt::Debug,
    C: BulkConsumer<Repeated = u8, Stopped = E>,
{
    type Ok = ();
    type Error = EncodeError<E>;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), EncodeError<E>>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!();
        // if !self.output.ends_with('[') {
        //     self.output += ",";
        // }
        // value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError<E>> {
        unimplemented!();
        // self.output += "]";
        // Ok(())
    }
}

// Same thing but for tuple structs.
impl<'a, 's, C, E> ser::SerializeTupleStruct for &'a mut VVSerializer<'s, C>
where
    E: fmt::Display + fmt::Debug,
    C: BulkConsumer<Repeated = u8, Stopped = E>,
{
    type Ok = ();
    type Error = EncodeError<E>;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), EncodeError<E>>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!();
        // if !self.output.ends_with('[') {
        //     self.output += ",";
        // }
        // value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError<E>> {
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
impl<'a, 's, C, E> ser::SerializeTupleVariant for &'a mut VVSerializer<'s, C>
where
    E: fmt::Display + fmt::Debug,
    C: BulkConsumer<Repeated = u8, Stopped = E>,
{
    type Ok = ();
    type Error = EncodeError<E>;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), EncodeError<E>>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!();
        // if !self.output.ends_with('[') {
        //     self.output += ",";
        // }
        // value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError<E>> {
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
impl<'a, 's, C, E> ser::SerializeMap for &'a mut VVSerializer<'s, C>
where
    E: fmt::Display + fmt::Debug,
    C: BulkConsumer<Repeated = u8, Stopped = E>,
{
    type Ok = ();
    type Error = EncodeError<E>;

    // The Serde data model allows map keys to be any serializable type. JSON
    // only allows string keys so the implementation below will produce invalid
    // JSON if the key serializes as something other than a string.
    //
    // A real JSON serializer would need to validate that map keys are strings.
    // This can be done by using a different Serializer to serialize the key
    // (instead of `&mut **self`) and having that other serializer only
    // implement `serialize_str` and return an error on any other data type.
    fn serialize_key<T>(&mut self, key: &T) -> Result<(), EncodeError<E>>
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
    fn serialize_value<T>(&mut self, value: &T) -> Result<(), EncodeError<E>>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!();
        // self.output += ":";
        // value.serialize(&mut **self)
    }

    fn end(self) -> Result<(), EncodeError<E>> {
        unimplemented!();
        // self.output += "}";
        // Ok(())
    }
}

// Structs are like maps in which the keys are constrained to be compile-time
// constant strings.
impl<'a, 's, C, E> ser::SerializeStruct for &'a mut VVSerializer<'s, C>
where
    E: fmt::Display + fmt::Debug,
    C: BulkConsumer<Repeated = u8, Stopped = E>,
{
    type Ok = ();
    type Error = EncodeError<E>;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), EncodeError<E>>
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

    fn end(self) -> Result<(), EncodeError<E>> {
        unimplemented!();
        // self.output += "}";
        // Ok(())
    }
}

// Similar to `SerializeTupleVariant`, here the `end` method is responsible for
// closing both of the curly braces opened by `serialize_struct_variant`.
impl<'a, 's, C, E> ser::SerializeStructVariant for &'a mut VVSerializer<'s, C>
where
    E: fmt::Display + fmt::Debug,
    C: BulkConsumer<Repeated = u8, Stopped = E>,
{
    type Ok = ();
    type Error = EncodeError<E>;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), EncodeError<E>>
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

    fn end(self) -> Result<(), EncodeError<E>> {
        unimplemented!();
        // self.output += "}}";
        // Ok(())
    }
}
