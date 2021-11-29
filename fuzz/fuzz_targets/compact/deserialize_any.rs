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
            let mut canonic = VVDeserializer::new(&input[..], true);
            let mut compact = VVDeserializer::new(&input[..], false);

            let is_compact = Value::deserialize(&mut compact).is_ok();
            let is_canonic = Value::deserialize(&mut canonic).is_ok();
            if is_canonic { assert!(is_compact) }
        }
        _ => {}
    }
});
