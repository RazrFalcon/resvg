use std::fmt;


#[derive(Clone, Copy, PartialEq)]
struct MStr<'a>(&'a str);

impl<'a> fmt::Debug for MStr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

macro_rules! test {
    ($name:ident, $keep_named_groups:expr, $input:expr, $output:expr) => {
        #[test]
        fn $name() {
            let re_opt = usvg::Options {
                keep_named_groups: $keep_named_groups,
                .. usvg::Options::default()
            };
            let tree = usvg::Tree::from_str($input, &re_opt).unwrap();

            let xml_opt = usvg::XmlOptions {
                use_single_quote: true,
                indent: usvg::XmlIndent::Spaces(4),
                attributes_indent: usvg::XmlIndent::Spaces(4),
            };

            assert_eq!(MStr(&tree.to_string(xml_opt)), MStr($output));
        }
    };
}

test!(minimal, false,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <rect width='10' height='10'/>
</svg>
",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs/>
    <path
        fill='#000000'
        stroke='none'
        d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
</svg>
");

test!(groups, false,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <g>
        <g>
            <rect width='10' height='10'/>
        </g>
    </g>
</svg>
",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs/>
    <path
        fill='#000000'
        stroke='none'
        d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
</svg>
");

test!(clippath_with_invalid_child, false,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <clipPath id='clip1'>
        <rect/>
    </clipPath>
    <rect clip-path='url(#clip1)' width='10' height='10'/>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs/>
</svg>
");

test!(clippath_with_invalid_children, false,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <clipPath id='clip1'>
        <rect/>
        <line/>
        <polyline/>
        <polygon/>
        <circle/>
        <ellipse/>
        <path/>
    </clipPath>
    <rect clip-path='url(#clip1)' width='10' height='10'/>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs/>
</svg>
");

test!(group_clippath, false,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <clipPath id='clip1'>
        <rect width='10' height='10'/>
    </clipPath>
    <rect clip-path='url(#clip1)' width='10' height='10'/>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs>
        <clipPath
            id='clip1'>
            <path
                fill='#000000'
                stroke='none'
                d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
        </clipPath>
    </defs>
    <g
        clip-path='url(#clip1)'>
        <path
            fill='#000000'
            stroke='none'
            d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
    </g>
</svg>
");

// We remove all groups by default.
test!(ignore_groups_with_id, false,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <g id='some_group'>
        <rect width='10' height='10'/>
    </g>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs/>
    <path
        fill='#000000'
        stroke='none'
        d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
</svg>
");

test!(pattern_with_invalid_child, false,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <pattern id='patt1'>
        <rect/>
    </pattern>
    <rect fill='url(#patt1)' width='10' height='10'/>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs/>
    <path
        fill='none'
        stroke='none'
        visibility='hidden'
        d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
</svg>
");

test!(pattern_without_children, false,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <pattern id='patt1' patternUnits='userSpaceOnUse' width='20' height='40'/>
    <rect fill='url(#patt1)' width='10' height='10'/>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs/>
    <path
        fill='none'
        stroke='none'
        visibility='hidden'
        d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
</svg>
");

// TODO: add mask, filter, marker
// All supported elements should be listed.
// We keep id's even if `keep_named_groups` is disabled.
// ID on `svg`, `defs`, `stop` and `tspan` is ignored because they can't be rendered
test!(preserve_id, true,
"<svg id='svg1' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' viewBox='0 0 1 1'>
    <defs id='defs1'>
        <linearGradient id='lg1'>
            <stop id='stop1' offset='0' stop-color='white'/>
            <stop offset='1' stop-color='black'/>
        </linearGradient>
        <radialGradient id='rg1'>
            <stop offset='0' stop-color='white'/>
            <stop offset='1' stop-color='black'/>
        </radialGradient>
        <clipPath id='clip1'>
            <rect id='rect2' width='10' height='10'/>
        </clipPath>
        <pattern id='patt1' width='1' height='1'>
            <rect width='10' height='10'/>
        </pattern>
    </defs>
    <rect id='rect1' fill='url(#lg1)' stroke='url(#rg1)' clip-path='url(#clip1)' width='10' height='10'/>
    <path id='path1' fill='url(#patt1)' d='M 10 20 30 40'/>
    <use id='use1' xlink:href='#path1'/>
    <image id='image1' width='1' height='1' xlink:href='data:image/png;base64,
        iVBORw0KGgoAAAANSUhEUgAAABAAAAAQAQMAAAAlPW0iAAAAB3RJTUUH4gMLDwAjrsLbtwAAAAlw
        SFlzAAAuIwAALiMBeKU/dgAAABl0RVh0Q29tbWVudABDcmVhdGVkIHdpdGggR0lNUFeBDhcAAAAG
        UExURQAA/xjQP14JpdQAAAABYktHRACIBR1IAAAAFklEQVR42mMAgvp/IJTAhgdB1ADVAgDvdAnx
        N1Ib1gAAAABJRU5ErkJggg=='/>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:xlink='http://www.w3.org/1999/xlink'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs>
        <clipPath
            id='clip1'>
            <path
                id='rect2'
                fill='#000000'
                stroke='none'
                d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
        </clipPath>
        <linearGradient
            id='lg1'
            x1='0'
            y1='0'
            x2='1'
            y2='0'>
            <stop
                offset='0'
                stop-color='#ffffff'/>
            <stop
                offset='1'
                stop-color='#000000'/>
        </linearGradient>
        <radialGradient
            id='rg1'
            cx='0.5'
            cy='0.5'
            r='0.5'
            fx='0.5'
            fy='0.5'>
            <stop
                offset='0'
                stop-color='#ffffff'/>
            <stop
                offset='1'
                stop-color='#000000'/>
        </radialGradient>
        <pattern
            id='patt1'
            x='0'
            y='0'
            width='1'
            height='1'>
            <path
                fill='#000000'
                stroke='none'
                d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
        </pattern>
    </defs>
    <g
        clip-path='url(#clip1)'>
        <path
            id='rect1'
            fill='url(#lg1)'
            stroke='url(#rg1)'
            d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
    </g>
    <path
        id='path1'
        fill='url(#patt1)'
        stroke='none'
        d='M 10 20 L 30 40'/>
    <g
        id='use1'>
        <path
            fill='url(#patt1)'
            stroke='none'
            d='M 10 20 L 30 40'/>
    </g>
    <image
        id='image1'
        x='0'
        y='0'
        width='1'
        height='1'
        xlink:href='data:image/png;base64, \
iVBORw0KGgoAAAANSUhEUgAAABAAAAAQAQMAAAAlPW0iAAAAB3RJTUUH4gMLDwAj\
rsLbtwAAAAlwSFlzAAAuIwAALiMBeKU/dgAAABl0RVh0Q29tbWVudABDcmVhdGVk\
IHdpdGggR0lNUFeBDhcAAAAGUExURQAA/xjQP14JpdQAAAABYktHRACIBR1IAAAA\
FklEQVR42mMAgvp/IJTAhgdB1ADVAgDvdAnxN1Ib1gAAAABJRU5ErkJggg=='/>
</svg>
");

// No need to keep empty groups even if `keep_named_groups` is enabled.
test!(ignore_empty_groups_with_id, true,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <g id='some_group'/>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs/>
</svg>
");

test!(keep_groups_with_id, true,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <g id='some_group'>
        <rect width='10' height='10'/>
    </g>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs/>
    <g
        id='some_group'>
        <path
            fill='#000000'
            stroke='none'
            d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
    </g>
</svg>
");

test!(simplify_paths_1, false,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <path d='M 10 20 L 10 30 Z Z Z'/>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs/>
    <path
        fill='#000000'
        stroke='none'
        d='M 10 20 L 10 30 Z'/>
</svg>
");

test!(group_with_default_opacity, false,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <g opacity='1'>
        <path d='M 10 20 L 10 30'/>
        <path d='M 10 20 L 10 30'/>
    </g>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs/>
    <path
        fill='#000000'
        stroke='none'
        d='M 10 20 L 10 30'/>
    <path
        fill='#000000'
        stroke='none'
        d='M 10 20 L 10 30'/>
</svg>
");

test!(group_with_an_invalid_child, false,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <g opacity='0.5'>
        <rect/>
    </g>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs/>
</svg>
");

test!(nested_group_with_an_invalid_child, false,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <g opacity='0.5'>
        <g opacity='0.5'>
            <rect/>
        </g>
    </g>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs/>
</svg>
");

test!(simple_switch, false,
"<svg viewBox='0 0 1 1' xmlns='http://www.w3.org/2000/svg'>
    <switch>
        <rect id='rect1' x='20' y='20' width='160' height='160' fill='red'
              requiredExtensions='http://example.org/bogus'/>
        <rect id='rect2' x='20' y='20' width='160' height='160' fill='green'/>
        <rect id='rect3' x='20' y='20' width='160' height='160' fill='red'/>
    </switch>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs/>
    <path
        id='rect2'
        fill='#008000'
        stroke='none'
        d='M 20 20 L 180 20 L 180 180 L 20 180 Z'/>
</svg>
");

test!(switch_with_opacity, false,
"<svg viewBox='0 0 1 1' xmlns='http://www.w3.org/2000/svg'>
    <switch opacity='0.5'>
        <rect id='rect1' x='20' y='20' width='160' height='160' fill='red'
              requiredExtensions='http://example.org/bogus'/>
        <rect id='rect2' x='20' y='20' width='160' height='160' fill='green'/>
        <rect id='rect3' x='20' y='20' width='160' height='160' fill='red'/>
    </switch>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs/>
    <g
        opacity='0.5'>
        <path
            id='rect2'
            fill='#008000'
            stroke='none'
            d='M 20 20 L 180 20 L 180 180 L 20 180 Z'/>
    </g>
</svg>
");

//// `fill-rule` cannot be set on `text`.
//test!(fill_rule_on_text, false,
//"<svg viewBox='0 0 1 1' xmlns='http://www.w3.org/2000/svg'>
//    <text fill-rule='evenodd'>_</text>
//</svg>",
//"<svg
//    xmlns='http://www.w3.org/2000/svg'
//    width='1'
//    height='1'
//    viewBox='0 0 1 1'
//    xmlns:usvg='https://github.com/RazrFalcon/resvg'
//    usvg:version='0.8.0'>
//    <defs/>
//    <path
//        d='M -0.09375 1.546875 L -0.09375 0.94921875 L 6.09375 0.94921875 L 6.09375 1.546875 Z'/>
//</svg>
//");

test!(fe_image_duplicates, false,
"<svg viewBox='0 0 1 1' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'>
    <defs>
        <filter id='filter1'>
            <feImage xlink:href='#rect1'/>
            <feImage xlink:href='#rect1'/>
        </filter>
        <rect id='rect1' width='120' height='120'/>
    </defs>
    <rect filter='url(#filter1)' width='120' height='120'/>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:xlink='http://www.w3.org/1999/xlink'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs>
        <filter
            id='filter1'
            x='-0.1'
            y='-0.1'
            width='1.2'
            height='1.2'>
            <feImage
                color-interpolation-filters='linearRGB'
                preserveAspectRatio='xMidYMid'
                image-rendering='optimizeQuality'
                xlink:href='#rect1'
                result='result1'/>
            <feImage
                color-interpolation-filters='linearRGB'
                preserveAspectRatio='xMidYMid'
                image-rendering='optimizeQuality'
                xlink:href='#rect1'
                result='result2'/>
        </filter>
        <path
            id='rect1'
            fill='#000000'
            stroke='none'
            d='M 0 0 L 120 0 L 120 120 L 0 120 Z'/>
    </defs>
    <g
        filter='url(#filter1)'>
        <path
            fill='#000000'
            stroke='none'
            d='M 0 0 L 120 0 L 120 120 L 0 120 Z'/>
    </g>
</svg>
");

test!(fe_diffuse_lighting_without_light_source, false,
"<svg viewBox='0 0 1 1' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'>
    <filter id='filter1'>
        <feDiffuseLighting/>
    </filter>
    <rect filter='url(#filter1)' width='120' height='120'/>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs>
        <filter
            id='filter1'
            x='-0.1'
            y='-0.1'
            width='1.2'
            height='1.2'>
            <feFlood
                color-interpolation-filters='linearRGB'
                flood-color='#000000'
                flood-opacity='0'
                result='result1'/>
        </filter>
    </defs>
    <g
        filter='url(#filter1)'>
        <path
            fill='#000000'
            stroke='none'
            d='M 0 0 L 120 0 L 120 120 L 0 120 Z'/>
    </g>
</svg>
");

test!(fe_specular_lighting_without_light_source, false,
"<svg viewBox='0 0 1 1' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'>
    <filter id='filter1'>
        <feSpecularLighting/>
    </filter>
    <rect filter='url(#filter1)' width='120' height='120'/>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs>
        <filter
            id='filter1'
            x='-0.1'
            y='-0.1'
            width='1.2'
            height='1.2'>
            <feFlood
                color-interpolation-filters='linearRGB'
                flood-color='#000000'
                flood-opacity='0'
                result='result1'/>
        </filter>
    </defs>
    <g
        filter='url(#filter1)'>
        <path
            fill='#000000'
            stroke='none'
            d='M 0 0 L 120 0 L 120 120 L 0 120 Z'/>
    </g>
</svg>
");

test!(fe_specular_lighting_with_invalid_specular_exponent, false,
"<svg viewBox='0 0 1 1' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'>
    <filter id='filter1'>
        <feSpecularLighting specularExponent='999'>
            <feDistantLight/>
        </feSpecularLighting>
    </filter>
    <rect filter='url(#filter1)' width='120' height='120'/>
</svg>",
"<svg
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns='http://www.w3.org/2000/svg'
    xmlns:usvg='https://github.com/RazrFalcon/resvg'
    usvg:version='0.8.0'>
    <defs>
        <filter
            id='filter1'
            x='-0.1'
            y='-0.1'
            width='1.2'
            height='1.2'>
            <feFlood
                color-interpolation-filters='linearRGB'
                flood-color='#000000'
                flood-opacity='0'
                result='result1'/>
        </filter>
    </defs>
    <g
        filter='url(#filter1)'>
        <path
            fill='#000000'
            stroke='none'
            d='M 0 0 L 120 0 L 120 120 L 0 120 Z'/>
    </g>
</svg>
");


macro_rules! test_size {
    ($name:ident, $input:expr, $expected:expr) => {
        #[test]
        fn $name() {
            use usvg::FuzzyEq;
            let tree = usvg::Tree::from_str($input, &usvg::Options::default()).unwrap();
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

macro_rules! test_size_err {
    ($name:ident, $input:expr) => {
        #[test]
        fn $name() {
            assert!(usvg::Tree::from_str($input, &usvg::Options::default()).is_err());
        }
    };
}

test_size_err!(size_detection_err_1,
    "<svg width='50%' height='100%' xmlns='http://www.w3.org/2000/svg'>");

test_size_err!(size_detection_err_2,
    "<svg width='0' height='0' viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'>");

//// Marker resolving should not produce a group.
//test!(marker_with_visible_overflow, false,
//"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
//    <marker id='marker1' overflow='visible'>
//        <rect width='10' height='10'/>
//    </marker>
//    <path d='M 10 10 L 20 20' marker-start='url(#marker1)'/>
//</svg>",
//"<svg
//    xmlns='http://www.w3.org/2000/svg'
//    width='1'
//    height='1'
//    viewBox='0 0 1 1'
//    xmlns:usvg='https://github.com/RazrFalcon/resvg'
//    usvg:version='0.8.0'>
//    <defs/>
//    <path
//        fill='#000000'
//        fill-opacity='1'
//        fill-rule='nonzero'
//        stroke='none'
//        visibility='visible'
//        d='M 10 10 L 20 20'/>
//    <path
//        fill='#000000'
//        fill-opacity='1'
//        fill-rule='nonzero'
//        stroke='none'
//        visibility='visible'
//        d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
//</svg>
//");
