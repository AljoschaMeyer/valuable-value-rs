//! This module provides [`TestType`](TestType), a type that uses all aspects of the serde data model and is intended for testing purposes.
use std::collections::BTreeMap;
use std::fmt;

use arbitrary::Arbitrary;
use serde::{Serialize, Serializer, Deserialize, Deserializer, de::{self, Visitor}};

#[derive(PartialEq, Eq, Serialize, Deserialize, Arbitrary, Debug)]
pub struct TestType {
    a_bool: bool,
    a_u8: u8,
    a_u16: u16,
    a_u32: u32,
    a_u64: u64,
    a_i8: i8,
    a_i16: i16,
    a_i32: i32,
    a_i64: i64,
    a_char: char,
    a_string: String,
    bytes0: Bytes,
    bytes1: Bytes,
    bytes2: Bytes,
    an_option: Option<u8>,
    unit: (),
    unit_struct: UnitStruct,
    empty_struct: EmptyStruct,
    small_struct: SmallStruct,
    bigger_struct: BiggerStruct,
    new_type_struct: NewTypeStruct,
    sequence0: Vec<i16>,
    sequence1: Vec<i16>,
    sequence2: Vec<i16>,
    map0: BTreeMap<u8, u8>,
    map1: BTreeMap<u8, u8>,
    map2: BTreeMap<u8, u8>,
    enum_a: TestEnum,
    enum_b: TestEnum,
    enum_c: TestEnum,
    enum_d: TestEnum,
    enum_e: TestEnum,
    enum_z: TestEnum,
    enum_y: TestEnum,
    tuple0: (),
    // tuple1: (u8),
    tuple2: (u8, u8),
    nested: Nested,
}

#[derive(PartialEq, Eq, Arbitrary, Debug)]
pub struct Bytes(Vec<u8>);

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

struct BytesVisitor;

impl<'de> Visitor<'de> for BytesVisitor {
    type Value = Bytes;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a Bytes test value")
    }

    fn visit_bytes<E: de::Error>(self, s: &[u8]) -> Result<Self::Value, E> {
        Ok(Bytes(s.into()))
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(BytesVisitor)
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Arbitrary, Debug)]
pub struct UnitStruct;

#[derive(PartialEq, Eq, Serialize, Deserialize, Arbitrary, Debug)]
pub struct EmptyStruct {}

#[derive(PartialEq, Eq, Serialize, Deserialize, Arbitrary, Debug)]
pub struct SmallStruct {
    pub foo: u8,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Arbitrary, Debug)]
pub struct BiggerStruct {
    pub foo: u8,
    pub bar: i8,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Arbitrary, Debug)]
pub struct Nested {
    pub foo: BiggerStruct,
    pub bar: (u8, u8),
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Arbitrary, Debug)]
pub struct NewTypeStruct(u8);

#[derive(PartialEq, Eq, Serialize, Deserialize, Arbitrary, Debug)]
pub enum TestEnum {
    A,
    B(u8),
    C(u8, u8),
    D { field: i8 },
    E { foo: i8, bar: u8 },
    Z(),
    Y {},
}

pub fn new() -> TestType {
    let mut map1 = BTreeMap::new();
    map1.insert(0, 0);

    let mut map2 = BTreeMap::new();
    map2.insert(0, 0);
    map2.insert(1, 1);

    TestType {
        a_bool: false,
        a_u8: 0,
        a_u16: 0,
        a_u32: 0,
        a_u64: 0,
        a_i8: 0,
        a_i16: 0,
        a_i32: 0,
        a_i64: 0,
        a_char: '@',
        a_string: "@@".to_string(),
        bytes0: Bytes(vec![]),
        bytes1: Bytes(vec![0]),
        bytes2: Bytes(vec![0, 1]),
        an_option: Some(0),
        unit: (),
        unit_struct: UnitStruct,
        empty_struct: EmptyStruct {},
        small_struct: SmallStruct { foo: 0 },
        bigger_struct: BiggerStruct { foo: 0, bar: 0 },
        new_type_struct: NewTypeStruct(0),
        sequence0: vec![],
        sequence1: vec![0],
        sequence2: vec![0, 1],
        map0: BTreeMap::new(),
        map1,
        map2,
        enum_a: TestEnum::A,
        enum_b: TestEnum::B(0),
        enum_c: TestEnum::C(0, 0),
        enum_d: TestEnum::D { field: 0},
        enum_e: TestEnum::E { foo: 0, bar: 0},
        enum_z: TestEnum::Z(),
        enum_y: TestEnum::Y {},
        tuple0: (),
        // tuple1: (0),
        tuple2: (0, 0),
        nested: Nested { foo: BiggerStruct { foo: 0, bar: 0 }, bar: (0, 0) }
    }
}
