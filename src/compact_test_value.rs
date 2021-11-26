use core::cmp::max;

use std::collections::{BTreeMap, BTreeSet};

use arbitrary::{Arbitrary, Unstructured};
use pretty_dtoa::{dtoa, FmtFloatConfig};

use crate::value::Value;

/// A valuable value of arbitrary shape, together with information on how to encode it. Intended for generating varied but valid encodings for testing purposes.
#[derive(Arbitrary, Debug)]
pub enum CompactTestValue {
    Nil,
    Bool(bool),
    Int(Int),
    Float(f64),
    ByteString(ByteString),
    Array(Array),
    Set(Set),
}

impl CompactTestValue {
    pub fn canonic(&self) -> bool {
        match self {
            CompactTestValue::Nil | CompactTestValue::Bool(..) => true,
            CompactTestValue::Int(v) => v.canonic(),
            CompactTestValue::Float(n) => {
                if n.is_nan() {
                    n.to_bits() == u64::MAX
                } else {
                    true
                }
            }
            CompactTestValue::ByteString(v) => v.canonic(),
            CompactTestValue::Array(v) => v.canonic(),
            CompactTestValue::Set(v) => v.canonic(),
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            CompactTestValue::Nil => Value::Nil,
            CompactTestValue::Bool(b) => Value::Bool(*b),
            CompactTestValue::Int(v) => v.to_value(),
            CompactTestValue::Float(n) => Value::Float(*n),
            CompactTestValue::ByteString(v) => v.to_value(),
            CompactTestValue::Array(v) => v.to_value(),
            CompactTestValue::Set(v) => v.to_value(),
        }
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            CompactTestValue::Nil => {
                out.push(0b000_00000);
            }
            CompactTestValue::Bool(b) => {
                out.push(if *b { 0b001_00001 } else { 0b001_00000 });
            }
            CompactTestValue::Float(n) => {
                out.push(0b010_00000);
                out.extend_from_slice(&n.to_bits().to_be_bytes());
            }
            CompactTestValue::Int(v) => v.encode(out),
            CompactTestValue::ByteString(v) => v.encode(out),
            CompactTestValue::Array(v) => v.encode(out),
            CompactTestValue::Set(v) => v.encode(out),
        }
    }
}

#[derive(Arbitrary, Debug)]
pub struct Int {
    n: i64,
    bytes: u8,
}

impl Int {
    pub fn canonic(&self) -> bool {
        if 0 <= self.n && self.n <= 27 {
            self.bytes <= 0
        } else if (i8::MIN as i64) <= self.n && self.n <= (i8::MAX as i64) {
            self.bytes <= 1
        } else if (i16::MIN as i64) <= self.n && self.n <= (i16::MAX as i64) {
            self.bytes <= 2
        } else if (i32::MIN as i64) <= self.n && self.n <= (i32::MAX as i64) {
            self.bytes <= 4
        } else {
            true
        }
    }

    pub fn to_value(&self) -> Value {
        Value::Int(self.n)
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        let mut bytes = self.bytes;

        if 0 <= self.n && self.n <= 27 {
            bytes = max(0, bytes);
        } else if (i8::MIN as i64) <= self.n && self.n <= (i8::MAX as i64) {
            bytes = max(1, bytes);
        } else if (i16::MIN as i64) <= self.n && self.n <= (i16::MAX as i64) {
            bytes = max(2, bytes);
        } else if (i32::MIN as i64) <= self.n && self.n <= (i32::MAX as i64) {
            bytes = max(4, bytes);
        } else {
            bytes = max(8, bytes);
        }

        if bytes == 3 {
            bytes = 2;
        } else if bytes >= 5 && bytes <= 7 {
            bytes = 4
        } else if bytes > 8 {
            bytes = 8;
        }

        if bytes == 0 {
            out.push(0b010_00000 ^ (self.n as u8));
        } else if bytes == 1 {
            out.push(0b010_11100);
            out.extend_from_slice(&(self.n as i8).to_be_bytes());
        } else if bytes == 2 {
            out.push(0b010_11101);
            out.extend_from_slice(&(self.n as i16).to_be_bytes());
        } else if bytes == 4 {
            out.push(0b010_11110);
            out.extend_from_slice(&(self.n as i32).to_be_bytes());
        } else if bytes == 8 {
            out.push(0b010_11111);
            out.extend_from_slice(&(self.n as i64).to_be_bytes());
        } else {
            unreachable!();
        }
    }
}

#[derive(Arbitrary, Debug)]
pub struct ByteString {
    elements: Vec<u8>,
    count_width: u8,
}

impl ByteString {
    pub fn canonic(&self) -> bool {
        false
    }

    pub fn to_value(&self) -> Value {
        let mut arr = Vec::with_capacity(self.elements.len());
        for v in self.elements.iter() {
            arr.push(Value::Int(*v as i64));
        }
        Value::Array(arr)
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        encode_count(self.elements.len(), self.count_width, 0b100_00000, out);
        for v in self.elements.iter() {
            out.push(*v);
        }
    }
}

#[derive(Arbitrary, Debug)]
pub struct Array {
    elements: Vec<CompactTestValue>,
    count_width: u8,
}

impl Array {
    pub fn canonic(&self) -> bool {
        let correct_width = canonic_width(self.elements.len(), self.count_width);
        return correct_width && self.elements.iter().all(|v| v.canonic());
    }

    pub fn to_value(&self) -> Value {
        let mut arr = Vec::with_capacity(self.elements.len());
        for v in self.elements.iter() {
            arr.push(v.to_value());
        }
        Value::Array(arr)
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        encode_count(self.elements.len(), self.count_width, 0b101_00000, out);
        for v in self.elements.iter() {
            v.encode(out);
        }
    }
}

#[derive(Arbitrary, Debug)]
pub struct Set {
    elements: Vec<CompactTestValue>,
    count_width: u8,
}

impl Set {
    pub fn canonic(&self) -> bool {
        false
    }

    pub fn to_value(&self) -> Value {
        let mut m = BTreeMap::new();
        for v in self.elements.iter() {
            m.insert(v.to_value(), Value::Nil);
        }
        Value::Map(m)
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        encode_count(self.elements.len(), self.count_width, 0b110_00000, out);
        for v in self.elements.iter() {
            v.encode(out);
        }
    }
}

#[derive(Arbitrary, Debug)]
pub struct Map {
    elements: Vec<(CompactTestValue, CompactTestValue)>,
    count_width: u8,
}

impl Map {
    pub fn canonic(&self) -> bool {
        unimplemented!()
    }

    // pub fn to_value(&self) -> Value {
    //     let mut m = BTreeMap::new();
    //     for v in self.elements.iter() {
    //         m.insert(v.to_value(), Value::Nil);
    //     }
    //     Value::Map(m)
    // }
    //
    // pub fn encode(&self, out: &mut Vec<u8>) {
    //     encode_count(self.elements.len(), self.count_width, 0b110_00000, out);
    //     for v in self.elements.iter() {
    //         v.encode(out);
    //     }
    // }
}

fn canonic_width(n: usize, width: u8) -> bool {
    if n <= 27 {
        width <= 0
    } else if n <= (u8::MAX as usize) {
        width <= 1
    } else if n <= (u16::MAX as usize) {
        width <= 2
    } else if n <= (u16::MAX as usize) {
        width <= 4
    } else {
        true
    }
}

fn encode_count(n: usize, mut width: u8, mask: u8, out: &mut Vec<u8>) {
    if n <= 27 {
        width = max(0, width);
    } else if n <= u8::MAX as usize {
        width = max(1, width);
    } else if n <= u16::MAX as usize {
        width = max(2, width);
    } else if n <= u32::MAX as usize {
        width = max(4, width);
    } else {
        width = max(8, width);
    }

    if width == 3 {
        width = 2;
    } else if width >= 5 && width <= 7 {
        width = 4
    } else if width > 8 {
        width = 8;
    }

    if width == 0 {
        out.push(mask ^ (n as u8));
    } else if width == 1 {
        out.push(mask);
        out.extend_from_slice(&(n as u8).to_be_bytes());
    } else if width == 2 {
        out.push(mask);
        out.extend_from_slice(&(n as u16).to_be_bytes());
    } else if width == 4 {
        out.push(mask);
        out.extend_from_slice(&(n as u32).to_be_bytes());
    } else if width == 8 {
        out.push(mask);
        out.extend_from_slice(&(n as u64).to_be_bytes());
    } else {
        unreachable!();
    }
}
