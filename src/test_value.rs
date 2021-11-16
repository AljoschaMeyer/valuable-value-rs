use arbitrary::{Arbitrary, Unstructured};

use crate::value::Value;

/// A valuable value of arbitrary shape, together with information on how to encode it. Intended for generating varied but valid encodings for testing purposes.
#[derive(Arbitrary, Debug)]
pub enum TestValue {
    Nil(Spaces, Nil),
}

impl TestValue {
    pub fn canonic(&self) -> bool {
        match self {
            TestValue::Nil(s, v) => s.canonic() && v.canonic(),
        }
    }

    pub fn human(&self) -> bool {
        match self {
            TestValue::Nil(s, v) => s.human() && v.human(),
        }
    }

    pub fn compact(&self) -> bool {
        match self {
            TestValue::Nil(s, v) => s.compact() && v.compact(),
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            TestValue::Nil(_, v) => v.to_value(),
        }
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            TestValue::Nil(s, v) => {
                s.encode(out);
                v.encode(out);
            }
        }
    }
}

#[derive(Arbitrary, Debug)]
pub struct Spaces(Vec<Space>);

impl Spaces {
    pub fn canonic(&self) -> bool {
        self.0.len() == 0
    }

    pub fn human(&self) -> bool {
        true
    }

    pub fn compact(&self) -> bool {
        self.0.len() == 0
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        for s in &self.0 {
            s.0.encode(out);
        }
    }
}

#[derive(Debug)]
pub struct Space(Space_);

impl<'a> Arbitrary<'a> for Space {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        match Space_::arbitrary(u)? {
            Space_::Tab => Ok(Space(Space_::Tab)),
            Space_::LF => Ok(Space(Space_::LF)),
            Space_::CR => Ok(Space(Space_::CR)),
            Space_::Space => Ok(Space(Space_::Space)),
            Space_::Comment(c) => {
                if c.contains("\n") {
                    Err(arbitrary::Error::IncorrectFormat)
                } else {
                    Ok(Space(Space_::Comment(c)))
                }
            }
        }
    }
}

#[derive(Arbitrary, Debug)]
pub enum Space_ {
    Tab,
    LF,
    CR,
    Space,
    /// Must not contain a newline character (0x0a).
    Comment(String),
}

impl Space_ {
    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Space_::Tab => out.push(0x09),
            Space_::LF => out.push(0x0a),
            Space_::CR => out.push(0x0d),
            Space_::Space => out.push(0x20),
            Space_::Comment(c) => {
                out.push('#' as u8);
                out.extend_from_slice(c.as_bytes());
                out.push(0x0a);
            }
        }
    }
}

#[derive(Arbitrary, Debug)]
pub enum Nil {
    Human,
    Compact,
}

impl Nil {
    pub fn canonic(&self) -> bool {
        match self {
            Nil::Human => false,
            Nil::Compact => true,
        }
    }

    pub fn human(&self) -> bool {
        match self {
            Nil::Human => true,
            Nil::Compact => false,
        }
    }

    pub fn compact(&self) -> bool {
        match self {
            Nil::Human => false,
            Nil::Compact => true,
        }
    }

    pub fn to_value(&self) -> Value {
        Value::Nil
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Nil::Human => out.extend_from_slice(b"nil"),
            Nil::Compact => out.push(0b1_010_1100),
        }
    }
}
