use serde::de::MapAccess;
use serde::ser::SerializeMap;
use serde::ser::SerializeSeq;
use core::cmp::{self, Ordering};
use Ordering::*;

use std::fmt;
use std::collections::BTreeMap;

use serde::{Serialize, Serializer, Deserialize, Deserializer, de::{self, Visitor, SeqAccess}};

/// Represents a valuable value of arbitrary shape.
///
/// The implementations of `PartialEq` and `Eq` adheres to the [equality relation](https://github.com/AljoschaMeyer/valuable-value#equality) of the evaluable value specification, and the implementations of `PartialOrd` and `Ord` adhere to the [linear order](https://github.com/AljoschaMeyer/valuable-value#linear-order) of the specification.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone)]
pub enum Value {
    Nil,
    Bool(bool),
    Float(f64),
    Int(i64),
    Array(Vec<Value>),
    Map(BTreeMap<Value, Value>),
}

use Value::*;

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Nil => f.write_str("nil"),
            Bool(b) => {
                if *b {
                    f.write_str("true")
                } else {
                    f.write_str("false")
                }
            }
            Int(n) => n.fmt(f),
            Float(n) => n.fmt(f),
            Array(v) => f.debug_list().entries(v).finish(),
            Map(m) => m.fmt(f),
        }
    }
}

impl PartialEq for Value {
    /// See https://github.com/AljoschaMeyer/valuable-value#equality
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Nil, Nil) => true,
            (Bool(b1), Bool(b2)) => b1 == b2,
            (Int(n1), Int(n2)) => n1 == n2,
            (Float(n1), Float(n2)) => n1.is_nan() && n2.is_nan() || n1.to_bits() == n2.to_bits(),
            (Array(v1), Array(v2)) => v1 == v2,
            (Map(m1), Map(m2)) => m1 == m2,
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
            (Nil, Nil) => Equal,

            (Nil, Bool(_)) => Less,
            (Bool(_), Nil) => Greater,
            (Bool(b1), Bool(b2)) => b1.cmp(b2),

            (Nil, Float(_)) | (Bool(_), Float(_)) => Less,
            (Float(_), Nil) | (Float(_), Bool(_)) => Greater,
            (Float(n1), Float(n2)) => {
                if n1.is_nan() && n2.is_nan() {
                    Equal
                } else if n1.is_nan() {
                    Less
                } else if n2.is_nan() {
                    Greater
                } else {
                    n1.total_cmp(n2)
                }
            }

            (Nil, Int(_)) | (Bool(_), Int(_)) | (Float(_), Int(_)) => Less,
            (Int(_), Nil) | (Int(_), Bool(_)) | (Int(_), Float(_)) => Greater,
            (Int(n1), Int(n2)) => n1.cmp(n2),

            (Nil, Array(_)) | (Bool(_), Array(_)) | (Float(_), Array(_)) | (Int(_), Array(_)) => Less,
            (Array(_), Nil) | (Array(_), Bool(_)) | (Array(_), Float(_)) | (Array(_), Int(_)) => Greater,
            (Array(v1), Array(v2)) => v1.cmp(v2),

            (Nil, Map(_)) | (Bool(_), Map(_)) | (Float(_), Map(_)) | (Int(_), Map(_)) | (Array(_), Map(_)) => Less,
            (Map(_), Nil) | (Map(_), Bool(_)) | (Map(_), Float(_)) | (Map(_), Int(_)) | (Map(_), Array(_)) => Greater,
            (Map(m1), Map(m2)) => {
                let mut es1 = m1.iter();
                let mut es2 = m2.iter();

                loop {
                    match (es1.next(), es2.next()) {
                        (None, None) => return Equal,
                        (None, Some(_)) => return Less,
                        (Some(_), None) => return Greater,
                        (Some((k1, v1)), Some((k2, v2))) => {
                            match k1.cmp(k2) {
                                Less => return Greater,
                                Greater => return Less,
                                Equal => {
                                    match v1.cmp(v2) {
                                        Equal => {}
                                        other => return other,
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Value {
    /// Implements the [meaningful partial order](https://github.com/AljoschaMeyer/valuable-value#a-meaningful-partial-order) on the valuable values.
    pub fn meaningful_partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Nil, Nil) | (Bool(_), Bool(_)) | (Int(_), Int(_)) | (Float(_), Float(_)) => Some(self.cmp(other)),
            (Array(v1), Array(v2)) => {
                let mut i = 0;
                let mut so_far = Equal;
                loop {
                    match (v1.get(i), v2.get(i), so_far) {
                        (Some(i1), Some(i2), Equal) => so_far = i1.meaningful_partial_cmp(i2)?,
                        (Some(i1), Some(i2), Less) => {
                            if let Greater = i1.meaningful_partial_cmp(i2)? { return None; }
                        }
                        (Some(i1), Some(i2), Greater) => {
                            if let Less = i1.meaningful_partial_cmp(i2)? { return None; }
                        }

                        (Some(_), None, Equal | Greater) => return Some(Greater),
                        (Some(_), None, Less) => return None,

                        (None, Some(_), Equal | Less) => return Some(Less),
                        (None, Some(_), Greater) => return None,

                        (None, None, _) => return Some(so_far),
                    }

                    i += 1;
                }
            }
            (Map(m1), Map(m2)) => {
                let mut so_far = Equal;
                let mut es1 = m1.iter();
                let mut es2 = m2.iter();

                let mut e1 = es1.next();
                let mut e2 = es2.next();

                loop {
                    match (e1, e2, so_far) {
                        (Some((k1, v1)), Some((k2, v2)), Equal) => {
                            match k1.cmp(k2) {
                                Less => {
                                    so_far = Greater;
                                    e1 = es1.next();
                                }
                                Greater => {
                                    so_far = Less;
                                    e2 = es2.next();
                                }
                                Equal => {
                                    so_far = v1.meaningful_partial_cmp(v2)?;
                                    e1 = es1.next();
                                    e2 = es2.next();
                                }
                            }
                        }
                        (Some((k1, v1)), Some((k2, v2)), Less) => {
                            match k1.cmp(k2) {
                                Less => return None,
                                Greater => e2 = es2.next(),
                                Equal => {
                                    match v1.meaningful_partial_cmp(v2)? {
                                        Greater => return None,
                                        Less | Equal => {
                                            e1 = es1.next();
                                            e2 = es2.next();
                                        }
                                    }
                                }
                            }
                        }
                        (Some((k1, v1)), Some((k2, v2)), Greater) => {
                            match k1.cmp(k2) {
                                Less => e1 = es1.next(),
                                Greater => return None,
                                Equal => {
                                    match v1.meaningful_partial_cmp(v2)? {
                                        Less => return None,
                                        Equal | Greater => {
                                            e1 = es1.next();
                                            e2 = es2.next();
                                        }
                                    }
                                }
                            }
                        }

                        (Some(_), None, Equal | Greater) => return Some(Greater),
                        (Some(_), None, Less) => return None,

                        (None, Some(_), Equal | Less) => return Some(Less),
                        (None, Some(_), Greater) => return None,

                        (None, None, _) => return Some(so_far),
                    }
                }
            }
            _ => None,
        }
    }

    /// See https://github.com/AljoschaMeyer/valuable-value#a-meaningful-partial-order
    pub fn meaningful_lt(&self, other: &Self) -> bool {
        match (self, other) {
            (Nil, Nil) | (Bool(_), Bool(_)) | (Int(_), Int(_)) | (Float(_), Float(_)) => self.lt(other),
            (Array(_), Array(_)) | (Map(_), Map(_)) => {
                match self.meaningful_partial_cmp(other) {
                    Some(Less) => true,
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// See https://github.com/AljoschaMeyer/valuable-value#a-meaningful-partial-order
    pub fn meaningful_le(&self, other: &Self) -> bool {
        match (self, other) {
            (Nil, Nil) | (Bool(_), Bool(_)) | (Int(_), Int(_)) | (Float(_), Float(_)) => self.le(other),
            (Array(_), Array(_)) | (Map(_), Map(_)) => {
                match self.meaningful_partial_cmp(other) {
                    Some(Less | Equal) => true,
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// See https://github.com/AljoschaMeyer/valuable-value#a-meaningful-partial-order
    pub fn meaningful_gt(&self, other: &Self) -> bool {
        match (self, other) {
            (Nil, Nil) | (Bool(_), Bool(_)) | (Int(_), Int(_)) | (Float(_), Float(_)) => self.gt(other),
            (Array(_), Array(_)) | (Map(_), Map(_)) => {
                match self.meaningful_partial_cmp(other) {
                    Some(Greater) => true,
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// See https://github.com/AljoschaMeyer/valuable-value#a-meaningful-partial-order
    pub fn meaningful_ge(&self, other: &Self) -> bool {
        match (self, other) {
            (Nil, Nil) | (Bool(_), Bool(_)) | (Int(_), Int(_)) | (Float(_), Float(_)) => self.ge(other),
            (Array(_), Array(_)) | (Map(_), Map(_)) => {
                match self.meaningful_partial_cmp(other) {
                    Some(Greater | Equal) => true,
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// See https://github.com/AljoschaMeyer/valuable-value#a-meaningful-partial-order
    pub fn greatest_lower_bound(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Nil, Nil) => Some(Nil),
            (Bool(b1), Bool(b2)) => Some(Bool(*b1 && *b2)),
            (Int(n1), Int(n2)) => Some(Int(cmp::min(*n1, *n2))),
            (Float(n1), Float(n2)) => {
                if n1.is_nan() && n2.is_nan() {
                    Some(self.clone())
                } else if n1.is_nan() {
                    Some(self.clone())
                } else if n2.is_nan() {
                    Some(other.clone())
                } else {
                    if n1.total_cmp(n2) == Greater {
                        Some(other.clone())
                    } else {
                        Some(self.clone())
                    }
                }
            }
            (Array(v1), Array(v2)) => {
                let len = cmp::min(v1.len(), v2.len());
                let mut r = Vec::with_capacity(len);
                for i in 0..len {
                    match (v1.get(i), v2.get(i)) {
                        (Some(x1), Some(x2)) => {
                            match x1.meaningful_partial_cmp(x2)? {
                                Less | Equal => r.push(x1.clone()),
                                Greater => r.push(x2.clone()),
                            }
                        }
                        (Some(_), _) | (_, Some(_)) => return Some(Value::Array(r)),
                        (None, None) => unreachable!(),
                    }
                }
                return Some(Value::Array(r));
            }
            (Map(m1), Map(m2)) => {
                let mut r = BTreeMap::new();
                for (k, v1) in m1.iter() {
                    if let Some(v2) = m2.get(k) {
                        r.insert(k.clone(), v1.greatest_lower_bound(v2)?);
                    }
                }
                return Some(Map(r));
            }
            _ => None,
        }
    }

    /// See https://github.com/AljoschaMeyer/valuable-value#a-meaningful-partial-order
    pub fn least_upper_bound(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Nil, Nil) => Some(Nil),
            (Bool(b1), Bool(b2)) => Some(Bool(*b1 || *b2)),
            (Int(n1), Int(n2)) => Some(Int(cmp::max(*n1, *n2))),
            (Float(n1), Float(n2)) => {
                if n1.is_nan() && n2.is_nan() {
                    Some(self.clone())
                } else if n1.is_nan() {
                    Some(other.clone())
                } else if n2.is_nan() {
                    Some(self.clone())
                } else {
                    if n1.total_cmp(n2) == Less {
                        Some(other.clone())
                    } else {
                        Some(self.clone())
                    }
                }
            }
            (Array(v1), Array(v2)) => {
                let len = cmp::max(v1.len(), v2.len());
                let mut r = Vec::with_capacity(len);
                for i in 0..len {
                    match (v1.get(i), v2.get(i)) {
                        (Some(x1), Some(x2)) => {
                            match x1.meaningful_partial_cmp(x2)? {
                                Equal | Greater => r.push(x1.clone()),
                                Less => r.push(x2.clone()),
                            }
                        }
                        (Some(x), _) | (_, Some(x)) => r.push(x.clone()),
                        (None, None) => unreachable!(),
                    }
                }
                return Some(Value::Array(r));
            }
            (Map(m1), Map(m2)) => {
                let mut r = BTreeMap::new();
                for (k, v1) in m1.iter() {
                    if let Some(v2) = m2.get(k) {
                        r.insert(k.clone(), v1.least_upper_bound(v2)?);
                    } else {
                        r.insert(k.clone(), v1.clone());
                    }
                }
                for (k, v2) in m2.iter() {
                    if let Some(v1) = m1.get(k) {
                        r.insert(k.clone(), v2.least_upper_bound(v1)?);
                    } else {
                        r.insert(k.clone(), v2.clone());
                    }
                }
                return Some(Map(r));
            }
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
            Int(n) => serializer.serialize_i64(*n),
            Float(n) => serializer.serialize_f64(*n),
            Array(a) => {
                let mut s = serializer.serialize_seq(Some(a.len()))?;
                for v in a {
                    s.serialize_element(v)?;
                }
                s.end()
            }
            Map(m) => {
                let mut s = serializer.serialize_map(Some(m.len()))?;
                for (k, v) in m {
                    s.serialize_entry(k, v)?;
                }
                s.end()
            }
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

    fn visit_i8<E: de::Error>(self, n: i8) -> Result<Self::Value, E> {
        Ok(Int(n as i64))
    }

    fn visit_i16<E: de::Error>(self, n: i16) -> Result<Self::Value, E> {
        Ok(Int(n as i64))
    }

    fn visit_i32<E: de::Error>(self, n: i32) -> Result<Self::Value, E> {
        Ok(Int(n as i64))
    }

    fn visit_i64<E: de::Error>(self, n: i64) -> Result<Self::Value, E> {
        Ok(Int(n))
    }

    fn visit_u8<E: de::Error>(self, n: u8) -> Result<Self::Value, E> {
        Ok(Int(n as i64))
    }

    fn visit_u16<E: de::Error>(self, n: u16) -> Result<Self::Value, E> {
        Ok(Int(n as i64))
    }

    fn visit_u32<E: de::Error>(self, n: u32) -> Result<Self::Value, E> {
        Ok(Int(n as i64))
    }

    fn visit_u64<E: de::Error>(self, n: u64) -> Result<Self::Value, E> {
        Ok(Int(n as i64))
    }

    fn visit_f32<E: de::Error>(self, n: f32) -> Result<Self::Value, E> {
        Ok(Float(n as f64))
    }

    fn visit_f64<E: de::Error>(self, n: f64) -> Result<Self::Value, E> {
        Ok(Float(n))
    }

    fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
        self.visit_bytes(s.as_bytes())
    }

    fn visit_bytes<E: de::Error>(self, s: &[u8]) -> Result<Self::Value, E> {
        let mut v = Vec::with_capacity(s.len());
        for b in s {
            v.push(Int(*b as i64));
        }
        Ok(Array(v))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut v = match seq.size_hint() {
            Some(len) => Vec::with_capacity(len),
            None => Vec::new(),
        };

        while let Some(x) = seq.next_element()? {
            v.push(x);
        }

        return Ok(Array(v));
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut m = BTreeMap::new();

        while let Some((k, v)) = map.next_entry()? {
            m.insert(k, v);
        }

        return Ok(Map(m));
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
    fn eq() {
        assert!(Float(-0.0f64) != Float(0.0f64));
        let negative_nan = f64::from_bits(u64::MAX);
        let positive_nan = negative_nan.copysign(1.0);
        assert_eq!(Float(positive_nan), Float(negative_nan));
    }

    #[test]
    fn cmp() {
        assert!(Nil < Bool(false));

        assert!(Bool(false) < Bool(true));
        assert!(Bool(true) < Float(f64::NEG_INFINITY));

        assert!(Float(f64::NAN) < Float(f64::NEG_INFINITY));
        assert!(Float(f64::NEG_INFINITY) < Float(-1.0));
        assert!(Float(-1.0) < Float(-0.0));
        assert!(Float(-0.0) < Float(0.0));
        assert!(Float(0.0) < Float(1.0));
        assert!(Float(1.0) < Float(f64::INFINITY));

        assert!(Float(f64::NAN) < Int(i64::MIN));

        assert!(Int(i64::MAX) < Array(Vec::new()));

        assert!(Array(Vec::new()) < Map(BTreeMap::new()));
    }
}
