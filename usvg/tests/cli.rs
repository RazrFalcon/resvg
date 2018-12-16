extern crate assert_cli;
extern crate tempdir;
#[macro_use] extern crate pretty_assertions;

use std::fmt;

use tempdir::TempDir;

const APP_PATH: &str = "../target/debug/usvg";

#[test]
fn file_to_file() {
    let dir = TempDir::new("usvg").unwrap();
    let file_out = dir.path().join("test1.svg");
    let file_out = file_out.to_str().unwrap();

    let args = &[
        APP_PATH,
        "--indent=2",
        "--attrs-indent=3",
        "tests/images/test1-in.svg",
        file_out,
    ];

    assert_cli::Assert::command(args)
        .stdout().is("")
        .stderr().is("")
        .unwrap();

    cmp_files("tests/images/test1-out.svg", file_out);
}

#[test]
fn file_to_stdout() {
    let args = &[
        APP_PATH,
        "-c",
        "--indent=2",
        "--attrs-indent=3",
        "tests/images/test1-in.svg",
    ];

    assert_cli::Assert::command(args)
        .stdout().is(load_file("tests/images/test1-out.svg").as_str())
        .stderr().is("")
        .unwrap();
}

#[test]
fn stdin_to_file() {
    let dir = TempDir::new("usvg").unwrap();
    let file_out = dir.path().join("test1.svg");
    let file_out = file_out.to_str().unwrap();

    let args = &[
        APP_PATH,
        "--indent=2",
        "--attrs-indent=3",
        file_out,
        "-",
    ];

    assert_cli::Assert::command(args)
        .stdin(load_file("tests/images/test1-out.svg").as_str())
        .stdout().is("")
        .stderr().is("")
        .unwrap();

    cmp_files("tests/images/test1-out.svg", file_out);
}

#[test]
fn stdin_to_stdout() {
    let args = &[
        APP_PATH,
        "-c",
        "--indent=2",
        "--attrs-indent=3",
        "-",
    ];

    let data = load_file("tests/images/test1-out.svg");

    assert_cli::Assert::command(args)
        .stdin(data.as_str())
        .stdout().is(data.as_str())
        .stderr().is("")
        .unwrap();
}

#[derive(Clone, Copy, PartialEq)]
struct MStr<'a>(&'a str);

impl<'a> fmt::Debug for MStr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn cmp_files(path1: &str, path2: &str) {
    assert_eq!(MStr(&load_file(path1)), MStr(&load_file(path2)));
}

fn load_file(path: &str) -> String {
    use std::fs;
    use std::io::Read;

    let mut file = fs::File::open(path).unwrap();
    let length = file.metadata().unwrap().len() as usize;

    let mut s = String::with_capacity(length + 1);
    file.read_to_string(&mut s).unwrap();

    s
}

// TODO: we need log without line numbers, somehow
//#[test]
//fn svgdom_error_msg_1() {
//    let args = &[
//        APP_PATH,
//        "-c",
//        "tests/images/crosslink-err.svg",
//    ];
//
//    assert_cli::Assert::command(args)
//        .stdout().is(load_file("tests/images/default.svg"))
//        .stderr().is("Warning (in usvg:80): Failed to parse an SVG data cause element crosslink.\n\
//                      Warning (in usvg::preproc:99): Invalid SVG structure. The Document will be cleared.\n\
//                      Warning (in usvg::convert:63): Invalid SVG structure. An empty tree will be produced.\n")
//        .unwrap();
//}

//#[test]
//fn warn_msg_1() {
//    let args = &[
//        APP_PATH,
//        "-c",
//        "tests/images/invalid-attr-value-in.svg",
//    ];
//
//    assert_cli::Assert::command(args)
//        .stdout().is(load_file("tests/images/invalid-attr-value-out.svg"))
//        .stderr().is("Warning (in svgdom::parser:459): Attribute 'fill' has an invalid value: 'qwe'.\n")
//        .unwrap();
//}
