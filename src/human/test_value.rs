use std::collections::BTreeMap;

use arbitrary::Arbitrary;
use atm_parser_helper_common_syntax::testing::*;

use crate::value::Value;

/// A valuable value of arbitrary shape, together with information on how to encode it. Intended for generating varied but valid encodings for testing purposes.
#[derive(Arbitrary, Debug)]
pub enum TestValue {
    Nil(Spaces),
    Bool(Spaces, bool),
    Int(Spaces, Int),
    Float(Spaces, Float),
    ByteString(Spaces, ByteString),
    Utf8String(Spaces, Utf8String),
    Array(Spaces, Array),
    Set(Spaces, Set),
    Map(Spaces, Map),
}

impl TestValue {
    pub fn to_value(&self) -> Value {
        match self {
            TestValue::Nil(..) => Value::Nil,
            TestValue::Bool(_, b) => Value::Bool(*b),
            TestValue::Int(_, v) => Value::Int(v.to_value()),
            TestValue::Float(_, v) => Value::Float(v.to_value()),
            TestValue::ByteString(_, v) => Value::Array(v.to_value().into_iter().map(|b| Value::Int(b as i64)).collect()),
            TestValue::Utf8String(_, v) => Value::Array(v.to_value().into_bytes().into_iter().map(|b| Value::Int(b as i64)).collect()),
            TestValue::Array(_, v) => v.to_value(),
            TestValue::Set(_, v) => v.to_value(),
            TestValue::Map(_, v) => v.to_value(),
        }
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            TestValue::Nil(s) => {
                s.encode(out);
                out.extend_from_slice(b"nil");
            }
            TestValue::Bool(s, b) => {
                s.encode(out);
                out.extend_from_slice(if *b { b"true" } else { b"false" });
            }
            TestValue::Float(s, n) => {
                s.encode(out);
                n.encode(out);
            }
            TestValue::Int(s, v) => {
                s.encode(out);
                v.encode(out);
            }
            TestValue::ByteString(s, v) => {
                s.encode(out);
                v.encode(out);
            }
            TestValue::Utf8String(s, v) => {
                s.encode(out);
                v.encode(out);
            }
            TestValue::Array(s, v) => {
                s.encode(out);
                v.encode(out);
            }
            TestValue::Set(s, v) => {
                s.encode(out);
                v.encode(out);
            }
            TestValue::Map(s, v) => {
                s.encode(out);
                v.encode(out);
            }
        }
    }
}

#[derive(Arbitrary, Debug)]
pub struct Array {
    values: Vec<(Spaces, TestValue, Spaces)>,
    trailing_comma: Option<Spaces>,
}

impl Array {
    pub fn to_value(&self) -> Value {
        let mut arr = Vec::with_capacity(self.values.len());
        for (_, v, _) in self.values.iter() {
            arr.push(v.to_value());
        }
        Value::Array(arr)
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(b"[");
        let len = self.values.len();
        for (i, (s1, v, s2)) in self.values.iter().enumerate() {
            s1.encode(out);
            v.encode(out);
            s2.encode(out);

            if i + 1 == len {
                match &self.trailing_comma {
                    None => break,
                    Some(s) => {
                        out.push(',' as u8);
                        s.encode(out);
                    }
                }
            } else {
                out.push(',' as u8);
            }
        }
        out.push(']' as u8);
    }
}

#[derive(Arbitrary, Debug)]
pub struct Set {
    values: Vec<(Spaces, TestValue, Spaces)>,
    trailing_comma: Option<Spaces>,
}

impl Set {
    pub fn to_value(&self) -> Value {
        let mut m = BTreeMap::new();
        for (_, v, _) in self.values.iter() {
            m.insert(v.to_value(), Value::Nil);
        }
        Value::Map(m)
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(b"@{");
        let len = self.values.len();
        for (i, (s1, v, s2)) in self.values.iter().enumerate() {
            s1.encode(out);
            v.encode(out);
            s2.encode(out);

            if i + 1 == len {
                match &self.trailing_comma {
                    None => break,
                    Some(s) => {
                        out.push(',' as u8);
                        s.encode(out);
                    }
                }
            } else {
                out.push(',' as u8);
            }
        }
        out.push('}' as u8);
    }
}

#[derive(Arbitrary, Debug)]
pub struct Map {
    values: Vec<(Spaces, TestValue, Spaces, Spaces, TestValue, Spaces)>,
    trailing_comma: Option<Spaces>,
}

impl Map {
    pub fn to_value(&self) -> Value {
        let mut m = BTreeMap::new();
        for (_, k, _, _, v, _) in self.values.iter() {
            m.insert(k.to_value(), v.to_value());
        }
        Value::Map(m)
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(b"{");
        let len = self.values.len();
        for (i, (s1, k, s2, s3, v, s4)) in self.values.iter().enumerate() {
            s1.encode(out);
            k.encode(out);
            s2.encode(out);
            out.push(':' as u8);
            s3.encode(out);
            v.encode(out);
            s4.encode(out);

            if i + 1 == len {
                match &self.trailing_comma {
                    None => break,
                    Some(s) => {
                        out.push(',' as u8);
                        s.encode(out);
                    }
                }
            } else {
                out.push(',' as u8);
            }
        }
        out.push('}' as u8);
    }
}
