#![no_main]
use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};

use serde::{Deserialize};

use valuable_value::parser_helper;
use valuable_value::{
    test_value::TestValue,
    value::Value,
    de::*,
    ser::*,
    de,
    ser,
};

fuzz_target!(|data: &[u8]| {
    match <(Value, usize)>::arbitrary(&mut Unstructured::new(data)) {
        Ok((v, indentation)) => {
            let enc_canonic = ser::to_vec(&v, Format::Canonic).unwrap();
            let mut canonic = VVDeserializer::new(&enc_canonic[..], Encoding::Canonic);
            assert_eq!(Value::deserialize(&mut canonic).unwrap(), v);

            let enc_human = ser::to_vec(&v, Format::HumanReadable(indentation)).unwrap();
            let mut human = VVDeserializer::new(&enc_human[..], Encoding::HumanReadable);
            assert_eq!(Value::deserialize(&mut human).unwrap(), v);

            let enc_human2 = ser::to_string(&v, indentation).unwrap().into_bytes();
            let mut human2 = VVDeserializer::new(&enc_human2[..], Encoding::HumanReadable);
            assert_eq!(Value::deserialize(&mut human2).unwrap(), v);
        }
        _ => {}
    }
});
