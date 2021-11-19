use core::cmp::max;

use std::collections::{BTreeMap, BTreeSet};

use arbitrary::{Arbitrary, Unstructured};

use crate::value::Value;

/// A valuable value of arbitrary shape, together with information on how to encode it. Intended for generating varied but valid encodings for testing purposes.
#[derive(Arbitrary, Debug)]
pub enum TestValue {
    Nil(Spaces, Nil),
    Bool(Spaces, Bool),
    Int(Spaces, Int),
}

impl TestValue {
    pub fn canonic(&self) -> bool {
        match self {
            TestValue::Nil(s, v) => s.canonic() && v.canonic(),
            TestValue::Bool(s, v) => s.canonic() && v.canonic(),
            TestValue::Int(s, v) => s.canonic() && v.canonic(),
        }
    }

    pub fn human(&self) -> bool {
        match self {
            TestValue::Nil(s, v) => s.human() && v.human(),
            TestValue::Bool(s, v) => s.human() && v.human(),
            TestValue::Int(s, v) => s.human() && v.human(),
        }
    }

    pub fn compact(&self) -> bool {
        match self {
            TestValue::Nil(s, v) => s.compact() && v.compact(),
            TestValue::Bool(s, v) => s.compact() && v.compact(),
            TestValue::Int(s, v) => s.compact() && v.compact(),
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            TestValue::Nil(_, v) => v.to_value(),
            TestValue::Bool(_, v) => v.to_value(),
            TestValue::Int(_, v) => v.to_value(),
        }
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            TestValue::Nil(s, v) => {
                s.encode(out);
                v.encode(out);
            }
            TestValue::Bool(s, v) => {
                s.encode(out);
                v.encode(out);
            }
            TestValue::Int(s, v) => {
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

#[derive(Arbitrary, Debug)]
pub enum Bool {
    Human(bool),
    Compact(bool),
}

impl Bool {
    pub fn canonic(&self) -> bool {
        match self {
            Bool::Human(_) => false,
            Bool::Compact(_) => true,
        }
    }

    pub fn human(&self) -> bool {
        match self {
            Bool::Human(_) => true,
            Bool::Compact(_) => false,
        }
    }

    pub fn compact(&self) -> bool {
        match self {
            Bool::Human(_) => false,
            Bool::Compact(_) => true,
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            Bool::Human(b) => Value::Bool(*b),
            Bool::Compact(b) => Value::Bool(*b),
        }
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Bool::Human(b) => {
                if *b {
                    out.extend_from_slice(b"true");
                } else {
                    out.extend_from_slice(b"false");
                }
            }
            Bool::Compact(b) => {
                if *b {
                    out.push(0b1_010_1110);
                } else {
                    out.push(0b1_010_1101);
                }
            }
        }
    }
}

#[derive(Arbitrary, Debug)]
pub enum Base {
    Binary,
    Hex(BTreeSet<usize> /* capitalize */),
    Decimal,
}

#[derive(Arbitrary, Debug)]
pub enum Int {
    Human {
        n: i64,
        explicit_sign: bool,
        base: Base,
        underscores: BTreeMap<usize, u8>, // keys are positions in the number encoding, values the number of inserted underscores
        leading_zeros: u8,
    },
    Compact {
        n: i64,
        bytes: u8,
    },
}

impl Int {
    pub fn canonic(&self) -> bool {
        match self {
            Int::Human { .. } => false,
            Int::Compact { n, bytes } => {
                if 0 <= *n && *n <= 11 {
                    *bytes <= 0
                } else if -128 <= *n && *n <= 127 {
                    *bytes <= 1
                } else if -32768 <= *n && *n <= 32767 {
                    *bytes <= 2
                } else if -2147483648 <= *n && *n <= 2147483647 {
                    *bytes <= 4
                } else {
                    true
                }
            }
        }
    }

    pub fn human(&self) -> bool {
        match self {
            Int::Human { .. } => true,
            Int::Compact { .. } => false,
        }
    }

    pub fn compact(&self) -> bool {
        match self {
            Int::Human { .. } => false,
            Int::Compact { .. } => true,
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            Int::Human { n, .. } => Value::Int(*n),
            Int::Compact { n, .. } => Value::Int(*n),
        }
    }

    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Int::Human {
                n,
                explicit_sign,
                base,
                underscores,
                leading_zeros,
            } => {
                let mut tmp = Vec::new();

                if *n < 0 {
                    out.push('-' as u8);
                } else if *explicit_sign {
                    if let Base::Decimal = base {
                        out.push('+' as u8);
                    }
                }

                if *n >= 0 {
                    match base {
                        Base::Binary => {
                            out.extend_from_slice(b"0b");
                        }
                        Base::Hex(_) => {
                            out.extend_from_slice(b"0x");
                        }
                        Base::Decimal => {}
                    }
                }

                for _ in 0..(*leading_zeros as usize) {
                    tmp.push('0' as u8);
                }

                if *n >= 0 {
                    match base {
                        Base::Binary => {
                            tmp.extend_from_slice(&format!("{:#b}", n).as_bytes()[2..]);
                        }
                        Base::Hex(capitalized) => {
                            tmp.extend_from_slice(&format!("{:#x}", n).as_bytes()[2..]);
                            let mut tmp2 = Vec::new();
                            for (i, c) in tmp.iter().enumerate() {
                                if capitalized.contains(&i) {
                                    tmp2.push(char::from_u32(*c as u32).unwrap().to_ascii_uppercase() as u8);
                                } else {
                                    tmp2.push(*c);
                                }
                            }
                            tmp = tmp2;
                        }
                        Base::Decimal => {
                            tmp.extend_from_slice(format!("{}", n).as_bytes());
                        }
                    }
                } else {
                    if *n == i64::MIN {
                        tmp.extend_from_slice(format!("{}", 9223372036854775808u64).as_bytes());
                    } else {
                        tmp.extend_from_slice(format!("{}", n.abs()).as_bytes());
                    }
                }

                for (i, b) in tmp.iter().enumerate() {
                    out.push(*b);
                    if let Some(m) = underscores.get(&i) {
                        for _ in 0..((*m) as usize) {
                            out.push('_' as u8);
                        }
                    }
                }
            }
            Int::Compact {
                n,
                mut bytes,
            } => {
                if 0 <= *n && *n <= 11 {
                    bytes = max(0, bytes);
                } else if -128 <= *n && *n <= 127 {
                    bytes = max(1, bytes);
                } else if -32768 <= *n && *n <= 32767 {
                    bytes = max(2, bytes);
                } else if -2147483648 <= *n && *n <= 2147483647 {
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
                    out.push(0b1_011_0000 ^ (*n as u8));
                } else if bytes == 1 {
                    out.push(0b1_011_1100);
                    out.extend_from_slice(&(*n as i8).to_be_bytes());
                } else if bytes == 2 {
                    out.push(0b1_011_1101);
                    out.extend_from_slice(&(*n as i16).to_be_bytes());
                } else if bytes == 4 {
                    out.push(0b1_011_1110);
                    out.extend_from_slice(&(*n as i32).to_be_bytes());
                } else if bytes == 8 {
                    out.push(0b1_011_1111);
                    out.extend_from_slice(&(*n as i64).to_be_bytes());
                } else {
                    unreachable!();
                }
            }
        }
    }
}
