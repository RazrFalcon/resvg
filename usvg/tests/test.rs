extern crate usvg;
extern crate svgdom;
#[macro_use] extern crate pretty_assertions;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate toml;
extern crate rustc_test;

// TODO: rewrite

use std::{env, fs, fmt};
use std::path::Path;

use svgdom::WriteBuffer;

use rustc_test::{TestDesc, TestDescAndFn, DynTestName, DynTestFn};

#[derive(Clone, Copy, PartialEq)]
struct MStr<'a>(&'a str);

impl<'a> fmt::Debug for MStr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Deserialize)]
struct TestData {
    keep_named_groups: Option<bool>,
    input: String,
    output: String,
}

#[test]
fn run() {
    let mut tests = vec![];

    for entry in fs::read_dir("tests/data").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let path = path.as_path();

        if path.extension().map(|v| v.to_str()) != Some(Some("xml")) {
            continue;
        }

        let file_name = path.file_stem().unwrap();
        let file_name = file_name.to_str().unwrap().to_owned();

//        if file_name != "pattern_without_children" {
//            continue;
//        }

        let test: TestData = toml::from_str(&load_file(path)).unwrap();

        tests.push(TestDescAndFn {
            desc: TestDesc::new(DynTestName(file_name)),
            testfn: DynTestFn(Box::new(move || actual_test(test.clone()))),
        });
    }

    let args: Vec<_> = env::args().collect();
    rustc_test::test_main(&args[..1], tests);
}

fn actual_test(test: TestData) {
    let re_opt = usvg::Options {
        keep_named_groups: test.keep_named_groups.unwrap_or(false),
        .. usvg::Options::default()
    };
    let tree = usvg::Tree::from_str(&test.input, &re_opt).unwrap();

    let dom_opt = svgdom::WriteOptions {
        attributes_indent: svgdom::Indent::Spaces(4),
        attributes_order: svgdom::AttributesOrder::Specification,
        .. svgdom::WriteOptions::default()
    };

    assert_eq!(MStr(&tree.to_svgdom().with_write_opt(&dom_opt).to_string()),
               MStr(&test.output));
}

fn load_file(path: &Path) -> String {
    use std::fs;
    use std::io::Read;

    let mut file = fs::File::open(path).unwrap();
    let length = file.metadata().unwrap().len() as usize;

    let mut s = String::with_capacity(length + 1);
    file.read_to_string(&mut s).unwrap();

    s
}
