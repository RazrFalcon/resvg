#![no_main]

#[macro_use] extern crate libfuzzer_sys;
extern crate svgrtypes;

use std::str::FromStr;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Must not panic.
        let _ = svgrtypes::Color::from_str(s);
    }
});
