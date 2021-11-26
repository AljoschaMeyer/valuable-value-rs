#![feature(total_cmp)]

pub mod parser_helper;

#[cfg(feature = "arbitrary")]
pub mod test_value;
#[cfg(feature = "arbitrary")]
pub mod compact_test_value;


pub mod de;
pub mod ser;





pub mod value;
pub mod compact;
