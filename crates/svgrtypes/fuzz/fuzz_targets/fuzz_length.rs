#![no_main]

#[macro_use] extern crate libfuzzer_sys;
extern crate svgrtypes;

use std::str;
use std::str::FromStr;

use svgrtypes::{Length, Error};

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = str::from_utf8(data) {
        if let Err(e) = Length::from_str(s) {
            match e {
                Error::InvalidNumber(_) => {}
                _ => panic!("{:?}", e),
            }
        }
    }
});
