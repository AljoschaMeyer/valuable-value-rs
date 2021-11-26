#![no_main]
use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};

use core::cmp::Ordering::*;

use valuable_value::value::Value;

fuzz_target!(|data: &[u8]| {
    match <(Value, Value, Value)>::arbitrary(&mut Unstructured::new(data)) {
        Ok((v, w, x)) => {
            if v == w {
                assert!(v <= w);
            }

            if v <= w && w <= v {
                assert_eq!(v, w);
            }

            if v <= w && w <= x {
                if !(v <= x) {
                    println!("v: {:?}", v);
                    println!("w: {:?}", w);
                    println!("x: {:?}", x);
                    panic!("transitivity violated, we do not have v <= w <= x");
                }
            }

            assert!(v <= w || w <= v);

            match v.cmp(&w) {
                Less => {
                    assert!(v < w);
                    assert!(v <= w);
                    assert!(!(v > w));
                    assert!(!(v >= w));
                    assert!(v != w);
                }

                Equal => {
                    assert!(!(v < w));
                    assert!(v <= w);
                    assert!(!(v > w));
                    assert!(v >= w);
                    assert!(v == w);
                }

                Greater => {
                    assert!(!(v < w));
                    assert!(!(v <= w));
                    assert!(v > w);
                    assert!(v >= w);
                    assert!(v != w);
                }
            }
        }
        _ => {}
    }
});
