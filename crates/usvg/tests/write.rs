use once_cell::sync::Lazy;

use usvg::TreeWriting;
use usvg_parser::TreeParsing;
use usvg_text_layout::TreeTextToPath;

static GLOBAL_FONTDB: Lazy<std::sync::Mutex<usvg_text_layout::fontdb::Database>> =
    Lazy::new(|| {
        let mut fontdb = usvg_text_layout::fontdb::Database::new();
        fontdb.load_fonts_dir("../resvg/tests/fonts");
        fontdb.set_serif_family("Noto Serif");
        fontdb.set_sans_serif_family("Noto Sans");
        fontdb.set_cursive_family("Yellowtail");
        fontdb.set_fantasy_family("Sedgwick Ave Display");
        fontdb.set_monospace_family("Noto Mono");
        std::sync::Mutex::new(fontdb)
    });

fn resave(name: &str) {
    resave_impl(name, None);
}

fn resave_with_prefix(name: &str, id_prefix: &str) {
    resave_impl(name, Some(id_prefix.to_string()));
}

fn resave_impl(name: &str, id_prefix: Option<String>) {
    let input_svg = std::fs::read_to_string(format!("tests/files/{}.svg", name)).unwrap();

    let tree = {
        let opt = usvg_parser::Options::default();
        let mut tree = usvg_tree::Tree::from_str(&input_svg, &opt).unwrap();
        let fontdb = GLOBAL_FONTDB.lock().unwrap();
        tree.convert_text(&fontdb);
        tree
    };
    let mut xml_opt = usvg::XmlOptions::default();
    xml_opt.id_prefix = id_prefix;
    xml_opt.coordinates_precision = 4; // Reduce noise and file size.
    xml_opt.transforms_precision = 4;
    let output_svg = tree.to_string(&xml_opt);

    // std::fs::write(format!("tests/files/{}-expected.svg", name), output_svg).unwrap();

    let expected_svg =
        std::fs::read_to_string(format!("tests/files/{}-expected.svg", name)).unwrap();
    // Do not use `assert_eq` because it produces an unreadable output.
    assert!(output_svg == expected_svg);
}

#[test]
fn path_simple_case() {
    resave("path-simple-case");
}

#[test]
fn ellipse_simple_case() {
    resave("ellipse-simple-case");
}

#[test]
fn text_simple_case() {
    resave("text-simple-case");
}

#[test]
fn preserve_id_filter() {
    resave("preserve-id-filter");
}

#[test]
fn generate_filter_id_function_v1() {
    resave("generate-id-filter-function-v1");
}

#[test]
fn generate_filter_id_function_v2() {
    resave("generate-id-filter-function-v2");
}

#[test]
fn filter_id_with_prefix() {
    resave_with_prefix("filter-id-with-prefix", "prefix-");
}

#[test]
fn preserve_id_clip_path_v1() {
    resave("preserve-id-clip-path-v1");
}

#[test]
fn preserve_id_clip_path_v2() {
    resave("preserve-id-clip-path-v2");
}

#[test]
fn generate_id_clip_path_for_symbol() {
    resave("generate-id-clip-path-for-symbol");
}

#[test]
fn clip_path_with_text() {
    resave("clip-path-with-text");
}

#[test]
fn clip_path_with_complex_text() {
    resave("clip-path-with-complex-text");
}

#[test]
fn text_with_generated_gradients() {
    resave("text-with-generated-gradients");
}
