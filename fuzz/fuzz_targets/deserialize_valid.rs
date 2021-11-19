#![no_main]
use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};

use serde::{Deserialize};

use valuable_value::parser_helper;
use valuable_value::{
    test_value::TestValue,
    value::Value,
    de::*,
    de,
};

fuzz_target!(|data: &[u8]| {
    match <TestValue>::arbitrary(&mut Unstructured::new(data)) {
        Ok(tv) => {
            let mut enc = Vec::new();
            tv.encode(&mut enc);
            let v = tv.to_value();

            let mut canonic = VVDeserializer::new(&enc[..], Encoding::Canonic);
            let mut compact = VVDeserializer::new(&enc[..], Encoding::Compact);
            let mut human_readable = VVDeserializer::new(&enc[..], Encoding::HumanReadable);
            let mut hybrid = VVDeserializer::new(&enc[..], Encoding::Hybrid);

            match Value::deserialize(&mut hybrid) {
                Err(e) => failure(&tv, &enc, &e, "Failed to deserialize valid hybrid encoding."),
                Ok(de_hybrid) => {
                    test_eq(&tv, &v, &enc, &de_hybrid);

                    if tv.human() {
                        match Value::deserialize(&mut human_readable) {
                            Err(e) => failure(&tv, &enc, &e, "Failed to deserialize human-readable encoding."),
                            Ok(de_human) => test_eq(&tv, &v, &enc, &de_human),
                        }
                    }

                    if tv.compact() {
                        match Value::deserialize(&mut compact) {
                            Err(e) => failure(&tv, &enc, &e, "Failed to deserialize compact encoding."),
                            Ok(de_compact) => {
                                test_eq(&tv, &v, &enc, &de_compact);

                                if tv.canonic() {
                                    match Value::deserialize(&mut canonic) {
                                        Err(e) => failure(&tv, &enc, &e, "Failed to deserialize canonic encoding."),
                                        Ok(de_canonic) => test_eq(&tv, &v, &enc, &de_canonic),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
});

fn failure(tv: &TestValue, enc: &Vec<u8>, e: &parser_helper::Error<de::DecodeError>, msg: &'static str) {
    println!("{:?}", tv);
    println!("{:?}", enc);
    println!("{:?}", e);
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
