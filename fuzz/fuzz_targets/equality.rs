#![no_main]
use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};

use valuable_value::value::Value;

fuzz_target!(|data: &[u8]| {
    match <(Value, Value, Value)>::arbitrary(&mut Unstructured::new(data)) {
        Ok((v, w, x)) => {
            assert!(v == v);

            if v == w {
                assert!(w == v);
            }

            if v == w && w == x {
                assert_eq!(v, w);
            }

            assert_eq!(v == w, !(v != w));
        }
        _ => {}
    }
});
