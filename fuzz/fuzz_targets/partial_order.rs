#![no_main]
use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};

use core::cmp::Ordering::*;

use valuable_value::value::Value;

fuzz_target!(|data: &[u8]| {
    match <(Value, Value, Value)>::arbitrary(&mut Unstructured::new(data)) {
        Ok((v, w, x)) => {
            if v == w {
                assert!(v.meaningful_le(&w));
            }

            if v.meaningful_le(&w) && w.meaningful_le(&v) {
                assert_eq!(v, w);
            }

            if v.meaningful_le(&w) && w.meaningful_le(&x) {
                assert!(v.meaningful_le(&x));
            }

            match v.meaningful_partial_cmp(&w) {
                None => {
                    assert!(!v.meaningful_lt(&w));
                    assert!(!v.meaningful_le(&w));
                    assert!(!v.meaningful_gt(&w));
                    assert!(!v.meaningful_ge(&w));
                    assert!(!v.eq(&w));

                    match v.greatest_lower_bound(&w) {
                        Some(glb) => assert!(glb != v && glb != w),
                        None => {}
                    }
                    match v.least_upper_bound(&w) {
                        Some(lub) => assert!(lub != v && lub != w),
                        None => {}
                    }
                }

                Some(Less) => {
                    assert!(v.meaningful_lt(&w));
                    assert!(v.meaningful_le(&w));
                    assert!(!v.meaningful_gt(&w));
                    assert!(!v.meaningful_ge(&w));
                    assert!(!v.eq(&w));
                    assert!(v < w);

                    match v.greatest_lower_bound(&w) {
                        Some(glb) => assert!(glb == v || glb == w),
                        None => {
                            println!("{:?} and {:?} must have a greatest_lower_bound", v, w);
                            panic!();
                        }
                    }
                    match v.least_upper_bound(&w) {
                        Some(glb) => assert!(glb == v || glb == w),
                        None => {
                            println!("{:?} and {:?} must have a least_upper_bound", v, w);
                            panic!();
                        }
                    }
                }

                Some(Equal) => {
                    assert!(!v.meaningful_lt(&w));
                    assert!(v.meaningful_le(&w));
                    assert!(!v.meaningful_gt(&w));
                    assert!(v.meaningful_ge(&w));
                    assert!(v.eq(&w));

                    match v.greatest_lower_bound(&w) {
                        Some(glb) => assert!(glb == v && glb == w),
                        None => {
                            println!("{:?} and {:?} must have a greatest_lower_bound", v, w);
                            panic!();
                        }
                    }
                    match v.least_upper_bound(&w) {
                        Some(glb) => assert!(glb == v && glb == w),
                        None => {
                            println!("{:?} and {:?} must have a least_upper_bound", v, w);
                            panic!();
                        }
                    }
                }

                Some(Greater) => {
                    assert!(!v.meaningful_lt(&w));
                    assert!(!v.meaningful_le(&w));
                    assert!(v.meaningful_gt(&w));
                    assert!(v.meaningful_ge(&w));
                    assert!(!v.eq(&w));
                    assert!(v > w);

                    match v.greatest_lower_bound(&w) {
                        Some(glb) => assert!(glb == v || glb == w),
                        None => {
                            println!("{:?} and {:?} must have a greatest_lower_bound", v, w);
                            panic!();
                        }
                    }
                    match v.least_upper_bound(&w) {
                        Some(glb) => assert!(glb == v || glb == w),
                        None => {
                            println!("{:?} and {:?} must have a least_upper_bound", v, w);
                            panic!();
                        }
                    }
                }
            }

            if let Some(glb) = v.greatest_lower_bound(&w) {
                assert!(v.meaningful_ge(&glb));
                assert!(w.meaningful_ge(&glb));
                assert_eq!(w.greatest_lower_bound(&v).unwrap(), glb);
            }

            if let Some(lub) = v.least_upper_bound(&w) {
                assert!(v.meaningful_le(&lub));
                assert!(w.meaningful_le(&lub));
                assert_eq!(w.least_upper_bound(&v).unwrap(), lub);
            }
        }
        _ => {}
    }
});
