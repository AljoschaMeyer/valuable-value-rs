#![no_main]
use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};

use serde::{Deserialize};
use atm_parser_helper::Error as ParseError;

use valuable_value::{
    value::Value,
    compact::{
        test_value::TestValue,
        *,
    }
};

fuzz_target!(|data: &[u8]| {
    match <TestValue>::arbitrary(&mut Unstructured::new(data)) {
        Ok(tv) => {
            let mut enc = Vec::new();
            tv.encode(&mut enc);
            let v = tv.to_value();

            let mut compact = VVDeserializer::new(&enc[..]);

            match Value::deserialize(&mut compact) {
                Err(e) => failure(&tv, &enc, &e, "Failed to deserialize compact encoding."),
                Ok(de_compact) => {
                    test_eq(&tv, &v, &enc, &de_compact);
                }
            }
        }
        _ => {}
    }
});

fn failure(tv: &TestValue, enc: &Vec<u8>, e: &ParseError<DecodeError>, msg: &'static str) {
    println!("TestValue: {:?}", tv);
    println!("encoded: {:?}", enc);
    println!("error: {:?}", e);
    panic!("{}", msg);
}

fn test_eq(tv: &TestValue, tvv: &Value, enc: &Vec<u8>, v: &Value) {
    if v != tvv {
        println!("TestValue: {:?}", tv);
        println!("encoded: {:?}", enc);
        println!("expected value: {:?}", tvv);
        println!("got: {:?}", v);
        panic!("failed roundtrip");
    }
}
