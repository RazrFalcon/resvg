extern crate assert_cli;
#[macro_use]
extern crate indoc;


use assert_cli::Assert;


const APP_PATH: &str = "target/debug/rendersvg";


#[test]
fn query_all() {
    let args = &[
        APP_PATH,
        "tests/images/bbox.svg",
        "--query-all",
    ];

    #[cfg(feature = "cairo-backend")]
    let output = indoc!("
        rect1,10,20,100,50
        rect2,7.5,92.5,105,55
        rect3,7.983,170,128.868,50
        rect4,7.182,242.5,133.868,55
        rect5,9.584,329.203,70.711,70.711
        long_text,10.281,441.953,550.594,10.047
        text1,200.703,23,55.406,22
        text2,200.875,102,32.172,28
        text3,202.311,178,57.692,22
        text4,195.703,243,65.406,32
        g1,350,20,100,50
        g2,350,95,120,70
    ");

    #[cfg(feature = "qt-backend")]
    let output = indoc!("
        rect1,10,20,100,50
        rect2,7.5,92.5,105,55
        rect3,7.983,170,128.868,50
        rect4,7.182,242.5,133.868,55
        rect5,9.584,329.203,70.711,70.711
        long_text,10.281,441.25,550.203,11.281
        text1,200.703,23.531,54.063,21.828
        text2,200.875,101.375,31.172,28.766
        text3,202.311,178.531,56.068,21.828
        text4,195.703,243.531,64.063,31.828
        g1,350,20,100,50
        g2,350,95,120,70
    ");

    Assert::command(args)
        .stdout().is(output)
        .stderr().is("")
        .unwrap();
}

// Check that all warnings are skipped during the ID querying.
// Some crates still can print to stdout/stderr, but we can't do anything about it.
#[test]
fn query_file_with_warnings() {
    let args = &[
        APP_PATH,
        "tests/images/bbox_with_warnings.svg",
        "--query-all",
    ];

    let output = indoc!("
        rect1,10,20,100,50
    ");

    Assert::command(args)
        .stdout().is(output)
        .stderr().is("")
        .unwrap();
}

#[test]
fn query_file_without_ids() {
    let args = &[
        APP_PATH,
        "tests/images/bbox_without_ids.svg",
        "--query-all",
    ];

    Assert::command(args)
        .stdout().is("")
        .stderr().is("Error: The file has no valid ID's.")
        .unwrap();
}
