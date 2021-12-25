#![no_main]
use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};

use serde::{Deserialize};

use valuable_value::{
    value::Value,
    human::*,
};

fuzz_target!(|data: &[u8]| {
    match <(Value, usize)>::arbitrary(&mut Unstructured::new(data)) {
        Ok((v, indentation)) => {
            let indentation = core::cmp::min(indentation, 4);
            let enc_human = to_vec(&v, indentation).unwrap();
            let mut human = VVDeserializer::new(&enc_human[..]);
            test_eq(&v, &mut human, &enc_human);
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
            println!("encoding string: {}", std::str::from_utf8(enc).unwrap());
            println!("error: {:?}", e);
            panic!();
        }
        Ok(dec) => {
            if v != &dec {
                println!("unequal original and decoded");
                println!("original: {:?}", v);
                println!("encoding: {:?}", enc);
                println!("encoding string: {}", std::str::from_utf8(enc).unwrap());
                println!("decoded: {:?}", dec);
                panic!();
            }
        }
    }
}
