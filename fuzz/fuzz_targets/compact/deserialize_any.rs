#![no_main]
use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};

use serde::{Deserialize};

use valuable_value::{
    value::Value,
    compact::*,
};

fuzz_target!(|data: &[u8]| {
    match <Vec<u8>>::arbitrary(&mut Unstructured::new(data)) {
        Ok(input) => {
            let mut compact = VVDeserializer::new(&input[..]);
            let _ = Value::deserialize(&mut compact);
        }
        _ => {}
    }
});
