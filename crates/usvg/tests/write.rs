use std::sync::Arc;

use once_cell::sync::Lazy;

static GLOBAL_FONTDB: Lazy<Arc<usvg::fontdb::Database>> = Lazy::new(|| {
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_fonts_dir("../resvg/tests/fonts");
    fontdb.set_serif_family("Noto Serif");
    fontdb.set_sans_serif_family("Noto Sans");
    fontdb.set_cursive_family("Yellowtail");
    fontdb.set_fantasy_family("Sedgwick Ave Display");
    fontdb.set_monospace_family("Noto Mono");
    Arc::new(fontdb)
});

fn resave(name: &str) {
    resave_impl(name, None, false);
}

fn resave_with_text(name: &str) {
    resave_impl(name, None, true);
}

fn resave_with_prefix(name: &str, id_prefix: &str) {
    resave_impl(name, Some(id_prefix.to_string()), false);
}

fn resave_impl(name: &str, id_prefix: Option<String>, preserve_text: bool) {
    let input_svg = std::fs::read_to_string(format!("tests/files/{}.svg", name)).unwrap();

    let tree = {
        let opt = usvg::Options {
            fontdb: GLOBAL_FONTDB.clone(),
            ..Default::default()
        };
        usvg::Tree::from_str(&input_svg, &opt).unwrap()
    };
    let mut xml_opt = usvg::WriteOptions::default();
    xml_opt.id_prefix = id_prefix;
    xml_opt.preserve_text = preserve_text;
    xml_opt.coordinates_precision = 4; // Reduce noise and file size.
    xml_opt.transforms_precision = 4;
    let output_svg = tree.to_string(&xml_opt);

    // std::fs::write(
    //     format!("tests/files/{}-expected.svg", name),
    //     output_svg.clone(),
    // )
    // .unwrap();

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
fn preserve_id_fe_image() {
    resave("preserve-id-fe-image");
}

#[test]
fn preserve_id_fe_image_with_opacity() {
    resave("preserve-id-fe-image-with-opacity");
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
fn filter_with_object_units_multi_use() {
    resave("filter-with-object-units-multi-use");
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
fn preserve_id_for_clip_path_in_pattern() {
    resave("preserve-id-for-clip-path-in-pattern");
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
fn clip_path_with_object_units_multi_use() {
    resave("clip-path-with-object-units-multi-use");
}

#[test]
fn mask_with_object_units_multi_use() {
    resave("mask-with-object-units-multi-use");
}

#[test]
fn text_with_generated_gradients() {
    resave("text-with-generated-gradients");
}

#[test]
fn preserve_text_multiple_font_families() {
    resave_with_text("preserve-text-multiple-font-families");
}

#[test]
fn preserve_text_on_path() {
    resave_with_text("preserve-text-on-path");
}

#[test]
fn preserve_text_in_clip_path() {
    resave_with_text("preserve-text-in-clip-path");
}

#[test]
fn preserve_text_in_mask() {
    resave_with_text("preserve-text-in-mask");
}

#[test]
fn preserve_text_in_pattern() {
    resave_with_text("preserve-text-in-pattern");
}

#[test]
fn preserve_text_simple_case() {
    resave("preserve-text-simple-case");
}

#[test]
fn preserve_text_with_dx_and_dy() {
    resave_with_text("preserve-text-with-dx-and-dy");
}

#[test]
fn preserve_text_with_rotate() {
    resave_with_text("preserve-text-with-rotate");
}

#[test]
fn preserve_text_with_complex_text_decoration() {
    resave_with_text("preserve-text-with-complex-text-decoration");
}

#[test]
fn preserve_text_with_nested_baseline_shift() {
    resave_with_text("preserve-text-with-nested-baseline-shift");
}
