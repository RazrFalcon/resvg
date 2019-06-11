extern crate afl;
extern crate usvg;

use std::str;

use afl::fuzz;

fn main() {
    let opt = usvg::Options::default();

    fuzz(|data| {
        if let Ok(text) = str::from_utf8(data) {
            let _ = usvg::Tree::from_str(text, &opt);
        }
    });
}
