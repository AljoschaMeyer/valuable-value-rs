#![no_main]
use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};

use serde::{Deserialize};

use valuable_value::{
    value::Value,
    de::*,
};

fuzz_target!(|data: &[u8]| {
    match <Vec<u8>>::arbitrary(&mut Unstructured::new(data)) {
        Ok(input) => {
            let mut canonic = VVDeserializer::new(&input[..], Encoding::Canonic);
            let mut compact = VVDeserializer::new(&input[..], Encoding::Compact);
            let mut human_readable = VVDeserializer::new(&input[..], Encoding::HumanReadable);
            let mut hybrid = VVDeserializer::new(&input[..], Encoding::Hybrid);

            let is_hybrid = Value::deserialize(&mut hybrid).is_ok();
            let is_human_readable = Value::deserialize(&mut human_readable).is_ok();
            let is_compact = Value::deserialize(&mut compact).is_ok();
            let is_canonic = Value::deserialize(&mut canonic).is_ok();

            if is_canonic { assert!(is_compact) }
            if is_compact { assert!(is_hybrid) }
            if is_human_readable { assert!(is_hybrid) }
        }
        _ => {}
    }
});
