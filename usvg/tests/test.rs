use std::fmt;


#[derive(Clone, Copy, PartialEq)]
struct MStr<'a>(&'a str);

impl<'a> fmt::Debug for MStr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

macro_rules! test {
    ($name:ident) => {
        #[test]
        fn $name() {
            let name = stringify!($name).replace("_", "-");
            let in_str = std::fs::read_to_string(format!("tests/files/{}-in.svg", name)).unwrap();
            let out_str = std::fs::read_to_string(format!("tests/files/{}-out.svg", name)).unwrap();

            let opt = usvg::Options::default();
            let tree = usvg::Tree::from_str(&in_str, &opt.to_ref()).unwrap();

            let xml_opt = usvg::XmlOptions {
                id_prefix: None,
                writer_opts: xmlwriter::Options {
                    use_single_quote: false,
                    indent: xmlwriter::Indent::Spaces(4),
                    attributes_indent: xmlwriter::Indent::Spaces(4),
                }
            };

            assert_eq!(MStr(&tree.to_string(&xml_opt)), MStr(&out_str));
        }
    };
}

macro_rules! test_preserve {
    ($name:ident) => {
        #[test]
        fn $name() {
            let name = stringify!($name).replace("_", "-");
            let in_str = std::fs::read_to_string(format!("tests/files/{}-in.svg", name)).unwrap();
            let out_str = std::fs::read_to_string(format!("tests/files/{}-out.svg", name)).unwrap();

            let re_opt = usvg::Options {
                keep_named_groups: true,
                .. usvg::Options::default()
            };
            let tree = usvg::Tree::from_str(&in_str, &re_opt.to_ref()).unwrap();

            let xml_opt = usvg::XmlOptions {
                id_prefix: None,
                writer_opts: xmlwriter::Options {
                    use_single_quote: false,
                    indent: xmlwriter::Indent::Spaces(4),
                    attributes_indent: xmlwriter::Indent::Spaces(4),
                }
            };

            assert_eq!(MStr(&tree.to_string(&xml_opt)), MStr(&out_str));
        }
    };
}

test!(minimal);
test!(groups);
test!(clippath_with_invalid_child);
test!(clippath_with_invalid_children);
test!(group_clippath);
test!(ignore_groups_with_id);
test!(pattern_with_invalid_child);
test!(pattern_without_children);
test!(simplify_paths);
test!(group_with_default_opacity);
test!(group_with_an_invalid_child);
test!(nested_group_with_an_invalid_child);
test!(simple_switch);
test!(switch_with_opacity);
test!(fe_image_duplicates);
test!(fe_image_with_invalid_link);
test!(fe_diffuse_lighting_without_light_source);
test!(fe_specular_lighting_without_light_source);
test!(fe_specular_lighting_with_invalid_specular_exponent);
// test!(fill_rule_on_text); // `fill-rule` cannot be set on `text`
// test!(marker_with_visible_overflow); // Marker resolving should not produce a group.

// TODO: add mask, filter, marker
// All supported elements should be listed.
// We keep id's even if `keep_named_groups` is disabled.
// ID on `svg`, `defs`, `stop` and `tspan` is ignored because they can't be rendered
test_preserve!(preserve_id);

// No need to keep empty groups even if `keep_named_groups` is enabled.
test_preserve!(ignore_empty_groups_with_id);

test_preserve!(keep_groups_with_id);


macro_rules! test_size {
    ($name:ident, $input:expr, $expected:expr) => {
        #[test]
        fn $name() {
            use usvg::FuzzyEq;
            let opt = usvg::Options::default();
            let tree = usvg::Tree::from_str($input, &opt.to_ref()).unwrap();
            assert!(tree.svg_node().size.fuzzy_eq(&$expected));
        }
    };
}

test_size!(size_detection_1,
    "<svg viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'>",
    usvg::Size::new(10.0, 20.0).unwrap()
);

test_size!(size_detection_2,
    "<svg width='30' height='40' viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'>",
    usvg::Size::new(30.0, 40.0).unwrap()
);

test_size!(size_detection_3,
    "<svg width='50%' height='100%' viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'>",
    usvg::Size::new(5.0, 20.0).unwrap()
);

test_size!(size_detection_4,
    "<svg xmlns='http://www.w3.org/2000/svg'><circle fill='#F4900C' cx='18' cy='18' r='18'/></svg>",
    usvg::Size::new(36.0, 36.0).unwrap()
);

#[test]
fn viewbox_detection() {
    use usvg::FuzzyEq;
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str("<svg xmlns='http://www.w3.org/2000/svg'><circle fill='#F4900C' cx='18' cy='18' r='18'/></svg>", &opt.to_ref()).unwrap();
    assert!(tree.svg_node().view_box.rect.fuzzy_eq(&usvg::Rect::new(0.0, 0.0, 36.0, 36.0).unwrap()));
}

macro_rules! test_size_err {
    ($name:ident, $input:expr) => {
        #[test]
        fn $name() {
            let opt = usvg::Options::default();
            assert!(usvg::Tree::from_str($input, &opt.to_ref()).is_err());
        }
    };
}

test_size_err!(size_detection_err,
    "<svg width='0' height='0' viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'>");
