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

            if let Ok(v) = Value::deserialize(&mut canonic) {
                let enc_canonic = to_vec(&v, true).unwrap();

                if enc_canonic != &input[..canonic.position()] {
                    println!("decoded value: {:?}", v);
                    println!("original input: {:?}", &input[..canonic.position()]);
                    println!("produced encoding: {:?}", enc_canonic);
                    panic!();
                }
            }
        }
        _ => {}
    }
});
