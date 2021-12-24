#![feature(total_cmp)]

#[cfg(feature = "arbitrary")]
pub mod test_value;
#[cfg(feature = "arbitrary")]
pub mod test_type;

pub mod value;
pub mod compact;
pub mod human;
mod helpers;
