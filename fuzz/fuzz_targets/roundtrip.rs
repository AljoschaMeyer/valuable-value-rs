#![no_main]
use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};

use serde::{Deserialize};

use valuable_value::{
    value::Value,
    de::*,
    ser::*,
    ser,
};

fuzz_target!(|data: &[u8]| {
    match <(Value, usize)>::arbitrary(&mut Unstructured::new(data)) {
        Ok((v, indentation)) => {
            let enc_canonic = ser::to_vec(&v, Format::Canonic).unwrap();
            let mut canonic = VVDeserializer::new(&enc_canonic[..], Encoding::Canonic);
            test_eq(&v, &mut canonic, &enc_canonic);
            // assert_eq!(Value::deserialize(&mut canonic).unwrap(), v);

            let enc_human = ser::to_vec(&v, Format::HumanReadable(indentation)).unwrap();
            let mut human = VVDeserializer::new(&enc_human[..], Encoding::HumanReadable);
            test_eq(&v, &mut human, &enc_human);
            // assert_eq!(Value::deserialize(&mut human).unwrap(), v);

            let enc_human2 = ser::to_string(&v, indentation).unwrap().into_bytes();
            let mut human2 = VVDeserializer::new(&enc_human2[..], Encoding::HumanReadable);
            test_eq(&v, &mut human2, &enc_human2);
            // assert_eq!(Value::deserialize(&mut human2).unwrap(), v);
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
