use core::cmp::Ordering;

/// Represents a valuable value of arbitrary shape.
pub enum Value {
    Nil,
}

use Value::*;

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Nil, Nil) => true,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Nil, Nil) => Ordering::Equal,
            _ => unreachable!(),
        }
    }
}
