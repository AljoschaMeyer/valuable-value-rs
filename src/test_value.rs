use crate::value::Value;

/// A valuable value of arbitrary shape, together with information on how to encode it.
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
            s.encode(out);
        }
    }
}

pub enum Space {
    Tab,
    LF,
    CR,
    Space,
    /// Must not contain a newline character (0x0a).
    Comment(String),
}

impl Space {
    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Space::Tab => out.push(0x09),
            Space::LF => out.push(0x0a),
            Space::CR => out.push(0x0d),
            Space::Space => out.push(0x20),
            Space::Comment(c) => {
                out.push('#' as u8);
                out.extend_from_slice(c.as_bytes());
            }
        }
    }
}

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
