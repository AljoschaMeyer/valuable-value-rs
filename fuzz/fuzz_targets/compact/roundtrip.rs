#![no_main]
use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};

use serde::{Deserialize};

use valuable_value::{
    value::Value,
    compact::*,
};

fuzz_target!(|data: &[u8]| {
    match <Value>::arbitrary(&mut Unstructured::new(data)) {
        Ok(v) => {
            let enc_compact = to_vec(&v).unwrap();
            let mut compact = VVDeserializer::new(&enc_compact[..]);
            test_eq(&v, &mut compact, &enc_compact);
        }
        _ => {}
    }
});

fn test_eq(v: &Value, de: &mut VVDeserializer, enc: &[u8]) {
    match Value::deserialize(de) {
        Err(e) => {
            println!("failed to deserialize");
            println!("original: {:?}", v);
            println!("encoding: {:?}", enc);
            println!("error: {:?}", e);
            panic!();
        }
        Ok(dec) => {
            if v != &dec {
                println!("unequal original and decoded");
                println!("original: {:?}", v);
                println!("encoding: {:?}", enc);
                println!("decoded: {:?}", dec);
                panic!();
            }
        }
    }
}
