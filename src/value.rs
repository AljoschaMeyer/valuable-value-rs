use core::cmp::Ordering;

use std::fmt;

use serde::{Serialize, Serializer, Deserialize, Deserializer, de::{self, Visitor}};

/// Represents a valuable value of arbitrary shape.
///
/// The implementations of `PartialEq` and `Eq` adheres to the [equality relation](https://github.com/AljoschaMeyer/valuable-value#equality) of the evaluable value specification, and the implementations of `PartialOrd` and `Ord` adhere to the [linear order](https://github.com/AljoschaMeyer/valuable-value#linear-order) of the specification.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum Value {
    Nil,
    Bool(bool),
}

use Value::*;

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Nil => f.debug_struct("nil").finish(),
            Bool(b) => {
                if *b {
                    f.debug_struct("true").finish()
                } else {
                    f.debug_struct("false").finish()
                }
            }
        }
    }
}

impl PartialEq for Value {
    /// See https://github.com/AljoschaMeyer/valuable-value#equality
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Nil, Nil) => true,
            (Bool(b1), Bool(b2)) => b1 == b2,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    /// See https://github.com/AljoschaMeyer/valuable-value#linear-order
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    /// See https://github.com/AljoschaMeyer/valuable-value#linear-order
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Nil, Nil) => Ordering::Equal,
            (Nil, Bool(_)) => Ordering::Less,
            (Bool(_), Nil) => Ordering::Greater,
            (Bool(b1), Bool(b2)) => b1.cmp(b2),
            _ => unreachable!(),
        }
    }
}

impl Value {
    /// Implements the [meaningful partial order](https://github.com/AljoschaMeyer/valuable-value#a-meaningful-partial-order) on the valuable values.
    pub fn meaningful_partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Nil, Nil) => Some(Ordering::Equal),
            (Bool(b1), Bool(b2)) => Some(b1.cmp(b2)),
            _ => None,
        }
    }

    /// See https://github.com/AljoschaMeyer/valuable-value#a-meaningful-partial-order
    pub fn meaningful_lt(&self, other: &Self) -> bool {
        match (self, other) {
            (Nil, Nil) => false,
            (Bool(b1), Bool(b2)) => b1.lt(b2),
            _ => false,
        }
    }

    /// See https://github.com/AljoschaMeyer/valuable-value#a-meaningful-partial-order
    pub fn meaningful_le(&self, other: &Self) -> bool {
        match (self, other) {
            (Nil, Nil) => true,
            (Bool(b1), Bool(b2)) => b1.le(b2),
            _ => false,
        }
    }

    /// See https://github.com/AljoschaMeyer/valuable-value#a-meaningful-partial-order
    pub fn meaningful_gt(&self, other: &Self) -> bool {
        match (self, other) {
            (Nil, Nil) => false,
            (Bool(b1), Bool(b2)) => b1.gt(b2),
            _ => false,
        }
    }

    /// See https://github.com/AljoschaMeyer/valuable-value#a-meaningful-partial-order
    pub fn meaningful_ge(&self, other: &Self) -> bool {
        match (self, other) {
            (Nil, Nil) => true,
            (Bool(b1), Bool(b2)) => b1.ge(b2),
            _ => false,
        }
    }

    /// See https://github.com/AljoschaMeyer/valuable-value#a-meaningful-partial-order
    pub fn greatest_lower_bound(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Nil, Nil) => Some(Nil),
            (Bool(b1), Bool(b2)) => Some(Bool(*b1 && *b2)),
            _ => None,
        }
    }

    /// See https://github.com/AljoschaMeyer/valuable-value#a-meaningful-partial-order
    pub fn least_upper_bound(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Nil, Nil) => Some(Nil),
            (Bool(b1), Bool(b2)) => Some(Bool(*b1 || *b2)),
            _ => None,
        }
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Nil => serializer.serialize_unit(),
            Bool(b) => serializer.serialize_bool(*b),
            _ => unimplemented!(),
        }
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a well-formed valuable value")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(Nil)
    }

    fn visit_bool<E: de::Error>(self, b: bool) -> Result<Self::Value, E> {
        Ok(Bool(b))
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmp() {
        assert!(Nil < Bool(false));
        assert!(Bool(false) < Bool(true));
    }
}
