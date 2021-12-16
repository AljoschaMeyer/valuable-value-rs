#![no_main]
use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};

use core::cmp::Ordering::*;

use valuable_value::value::Value;

fuzz_target!(|data: &[u8]| {
    match <(Value, Value, Value)>::arbitrary(&mut Unstructured::new(data)) {
        Ok((v, w, x)) => {
            if v == w {
                assert!(v.subvalue(&w));
            }

            if v.subvalue(&w) && w.subvalue(&v) {
                assert_eq!(v, w);
            }

            if v.subvalue(&w) && w.subvalue(&x) {
                assert!(v.subvalue(&x));
            }

            match v.subvalue_cmp(&w) {
                None => {
                    assert!(!v.strict_subvalue(&w));
                    assert!(!v.subvalue(&w));
                    assert!(!v.strict_supervalue(&w));
                    assert!(!v.supervalue(&w));
                    assert!(!v.eq(&w));

                    match v.greatest_common_subvalue(&w) {
                        Some(glb) => assert!(glb != v && glb != w),
                        None => {}
                    }
                    match v.least_common_supervalue(&w) {
                        Some(lub) => assert!(lub != v && lub != w),
                        None => {}
                    }
                }

                Some(Less) => {
                    assert!(v.strict_subvalue(&w));
                    assert!(v.subvalue(&w));
                    assert!(!v.strict_supervalue(&w));
                    assert!(!v.supervalue(&w));
                    assert!(!v.eq(&w));
                    test(v < w, &v, &w, "v.strict_subvalue(w) should imply v < w");

                    match v.greatest_common_subvalue(&w) {
                        Some(glb) => assert!(glb == v || glb == w),
                        None => {
                            println!("{:?} and {:?} must have a greatest_common_subvalue", v, w);
                            panic!();
                        }
                    }
                    match v.least_common_supervalue(&w) {
                        Some(glb) => assert!(glb == v || glb == w),
                        None => {
                            println!("{:?} and {:?} must have a least_common_supervalue", v, w);
                            panic!();
                        }
                    }
                }

                Some(Equal) => {
                    assert!(!v.strict_subvalue(&w));
                    assert!(v.subvalue(&w));
                    assert!(!v.strict_supervalue(&w));
                    assert!(v.supervalue(&w));
                    assert!(v.eq(&w));

                    match v.greatest_common_subvalue(&w) {
                        Some(glb) => assert!(glb == v && glb == w),
                        None => {
                            println!("{:?} and {:?} must have a greatest_common_subvalue", v, w);
                            panic!();
                        }
                    }
                    match v.least_common_supervalue(&w) {
                        Some(glb) => assert!(glb == v && glb == w),
                        None => {
                            println!("{:?} and {:?} must have a least_common_supervalue", v, w);
                            panic!();
                        }
                    }
                }

                Some(Greater) => {
                    assert!(!v.strict_subvalue(&w));
                    assert!(!v.subvalue(&w));
                    assert!(v.strict_supervalue(&w));
                    assert!(v.supervalue(&w));
                    assert!(!v.eq(&w));
                    test(v > w, &v, &w, "v.strict_supervalue(w) should imply v > w");

                    match v.greatest_common_subvalue(&w) {
                        Some(glb) => assert!(glb == v || glb == w),
                        None => {
                            println!("{:?} and {:?} must have a greatest_common_subvalue", v, w);
                            panic!();
                        }
                    }
                    match v.least_common_supervalue(&w) {
                        Some(glb) => assert!(glb == v || glb == w),
                        None => {
                            println!("{:?} and {:?} must have a least_common_supervalue", v, w);
                            panic!();
                        }
                    }
                }
            }

            if let Some(glb) = v.greatest_common_subvalue(&w) {
                assert!(v.supervalue(&glb));
                assert!(w.supervalue(&glb));
                assert_eq!(w.greatest_common_subvalue(&v).unwrap(), glb);
            }

            if let Some(lub) = v.least_common_supervalue(&w) {
                assert!(v.subvalue(&lub));
                assert!(w.subvalue(&lub));
                assert_eq!(w.least_common_supervalue(&v).unwrap(), lub);
            }
        }
        _ => {}
    }
});

fn test(b: bool, v: &Value, w: &Value, msg: &'static str) {
    if !b {
        println!("v: {:?}", v);
        println!("w: {:?}", w);
        panic!("{}", msg);
    }
}
