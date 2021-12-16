use std::collections::BTreeMap;

use arbitrary::Arbitrary;
use serde::{Serialize, Deserialize};

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
    bytes: Vec<u8>,
    an_option: Option<u8>,
    unit: (),
    unit_struct: UnitStruct,
    small_stuct: SmallStruct,
    new_type_struct: NewTypeStruct,
    sequence: Vec<i16>,
    map: BTreeMap<u8, u8>,
    an_enum: TestEnum,
    a_tuple: (i8, u8),
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Arbitrary, Debug)]
pub struct UnitStruct;

#[derive(PartialEq, Eq, Serialize, Deserialize, Arbitrary, Debug)]
pub struct SmallStruct {
    pub foo: u8,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Arbitrary, Debug)]
pub struct NewTypeStruct(u8);

#[derive(PartialEq, Eq, Serialize, Deserialize, Arbitrary, Debug)]
pub enum TestEnum {
    A,
    B(u8),
    C(u8, u8),
    D { field: i8 },
}
