//! An implementation of the [valuable value specification](https://github.com/AljoschaMeyer/valuable-value).
//!
//! Provides a general [`Value`](Value) type for working with valuable values of arbitrary shape, and [serde](https://serde.rs/) serializers and deserializers for both the [human-readable encoding](https://github.com/AljoschaMeyer/valuable-value#human-readable-encoding) and the [compact encoding](https://github.com/AljoschaMeyer/valuable-value#compact-encoding).
//!
//! There is no support for the [canonic encoding](https://github.com/AljoschaMeyer/valuable-value#canonic-encoding) because the serde API is not flexible enough to incorporate the required canonicity checks.
//!
//! Enable the `arbitrary` feature for an implementation of the [`Arbitrary`](arbitrary::Arbitrary) trait for the [`Value`](Value) type and further utilities for property testing.
#![feature(total_cmp)]

#[cfg(feature = "arbitrary")]
pub mod test_type;

mod value;
pub use value::Value;
pub mod compact;
pub mod human;
mod helpers;
