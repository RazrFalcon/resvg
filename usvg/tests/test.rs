extern crate usvg;
#[macro_use] extern crate pretty_assertions;

use usvg::svgdom;
use svgdom::WriteBuffer;

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

            let dom_opt = svgdom::WriteOptions {
                use_single_quote: true,
                attributes_indent: svgdom::Indent::Spaces(4),
                attributes_order: svgdom::AttributesOrder::Specification,
                .. svgdom::WriteOptions::default()
            };

            assert_eq!(MStr(&tree.to_svgdom().with_write_opt(&dom_opt).to_string()),
                       MStr($output));
        }
    };
}

test!(minimal, false,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <rect width='10' height='10'/>
</svg>
",
"<svg
    xmlns='http://www.w3.org/2000/svg'
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns:usvg='https://github.com/RazrFalcon/usvg'
    usvg:version='0.5.0'>
    <defs/>
    <path
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
    xmlns='http://www.w3.org/2000/svg'
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns:usvg='https://github.com/RazrFalcon/usvg'
    usvg:version='0.5.0'>
    <defs/>
    <path
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
    xmlns='http://www.w3.org/2000/svg'
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns:usvg='https://github.com/RazrFalcon/usvg'
    usvg:version='0.5.0'>
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
    xmlns='http://www.w3.org/2000/svg'
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns:usvg='https://github.com/RazrFalcon/usvg'
    usvg:version='0.5.0'>
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
    xmlns='http://www.w3.org/2000/svg'
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns:usvg='https://github.com/RazrFalcon/usvg'
    usvg:version='0.5.0'>
    <defs>
        <clipPath
            id='clip1'>
            <path
                d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
        </clipPath>
    </defs>
    <g
        clip-path='url(#clip1)'>
        <path
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
    xmlns='http://www.w3.org/2000/svg'
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns:usvg='https://github.com/RazrFalcon/usvg'
    usvg:version='0.5.0'>
    <defs/>
    <path
        id='some_group'
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
    xmlns='http://www.w3.org/2000/svg'
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns:usvg='https://github.com/RazrFalcon/usvg'
    usvg:version='0.5.0'>
    <defs/>
    <path
        fill='none'
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
    xmlns='http://www.w3.org/2000/svg'
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns:usvg='https://github.com/RazrFalcon/usvg'
    usvg:version='0.5.0'>
    <defs/>
    <path
        fill='none'
        visibility='hidden'
        d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
</svg>
");

// TODO: add mask, filter, marker
// All supported elements should be listed.
// We keep id's even if `keep_named_groups` is disabled.
// ID on `svg`, `defs`, `stop` is ignored because they can't be rendered
test!(preserve_id, false,
"<svg id='svg1' xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
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
    <text id='text1' font-family='Arial'>A</text>
    <text id='text2' font-family='Arial'><tspan id='tspan2'>A</tspan></text>
    <image id='image1' width='1' height='1' xlink:href='data:image/png;base64,
        iVBORw0KGgoAAAANSUhEUgAAABAAAAAQAQMAAAAlPW0iAAAAB3RJTUUH4gMLDwAjrsLbtwAAAAlw
        SFlzAAAuIwAALiMBeKU/dgAAABl0RVh0Q29tbWVudABDcmVhdGVkIHdpdGggR0lNUFeBDhcAAAAG
        UExURQAA/xjQP14JpdQAAAABYktHRACIBR1IAAAAFklEQVR42mMAgvp/IJTAhgdB1ADVAgDvdAnx
        N1Ib1gAAAABJRU5ErkJggg=='/>
</svg>",
"<svg
    xmlns='http://www.w3.org/2000/svg'
    xmlns:xlink='http://www.w3.org/1999/xlink'
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns:usvg='https://github.com/RazrFalcon/usvg'
    usvg:version='0.5.0'>
    <defs>
        <linearGradient
            id='lg1'
            x1='0'
            y1='0'
            x2='1'
            y2='0'>
            <stop
                stop-color='#ffffff'
                offset='0'/>
            <stop
                stop-color='#000000'
                offset='1'/>
        </linearGradient>
        <radialGradient
            id='rg1'
            cx='0.5'
            cy='0.5'
            r='0.5'
            fx='0.5'
            fy='0.5'>
            <stop
                stop-color='#ffffff'
                offset='0'/>
            <stop
                stop-color='#000000'
                offset='1'/>
        </radialGradient>
        <clipPath
            id='clip1'>
            <path
                id='rect2'
                d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
        </clipPath>
        <pattern
            id='patt1'
            x='0'
            y='0'
            width='1'
            height='1'>
            <path
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
        d='M 10 20 L 30 40'/>
    <path
        id='text1'
        d='M -0.017578125 0 L 3.28125 -8.58984375 L 4.505859375 -8.58984375 L 8.021484375 0 \
        L 6.7265625 0 L 5.724609375 -2.6015625 L 2.1328125 -2.6015625 L 1.189453125 0 Z \
        M 2.4609375 -3.52734375 L 5.373046875 -3.52734375 L 4.4765625 -5.90625 \
        C 4.20312488079 -6.62890648842 4.00000011921 -7.22265601158 3.8671875 -7.6875 \
        C 3.75781238079 -7.13671875 3.603515625 -6.58984351158 3.404296875 -6.046875 Z'/>
    <path
        id='text2'
        d='M -0.017578125 0 L 3.28125 -8.58984375 L 4.505859375 -8.58984375 L 8.021484375 0 \
        L 6.7265625 0 L 5.724609375 -2.6015625 L 2.1328125 -2.6015625 L 1.189453125 0 Z \
        M 2.4609375 -3.52734375 L 5.373046875 -3.52734375 L 4.4765625 -5.90625 \
        C 4.20312488079 -6.62890648842 4.00000011921 -7.22265601158 3.8671875 -7.6875 \
        C 3.75781238079 -7.13671875 3.603515625 -6.58984351158 3.404296875 -6.046875 Z'/>
    <image
        id='image1'
        x='0'
        y='0'
        width='1'
        height='1'
        xlink:href='data:image/png;base64, iVBORw0KGgoAAAANSUhEUgAAABAAAAAQAQMAAAAlPW0iAAAAB3RJT\
        UUH4gMLDwAjrsLbtwAAAAlwSFlzAAAuIwAALiMBeKU/dgAAABl0RVh0Q29tbWVudABDcmVhdGVkIHdpdGggR0lNU\
        FeBDhcAAAAGUExURQAA/xjQP14JpdQAAAABYktHRACIBR1IAAAAFklEQVR42mMAgvp/IJTAhgdB1ADVAgDvdAnxN\
        1Ib1gAAAABJRU5ErkJggg=='/>
</svg>
");

// No need to keep empty groups even if `keep_named_groups` is enabled.
test!(ignore_empty_groups_with_id, true,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <g id='some_group'/>
</svg>",
"<svg
    xmlns='http://www.w3.org/2000/svg'
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns:usvg='https://github.com/RazrFalcon/usvg'
    usvg:version='0.5.0'>
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
    xmlns='http://www.w3.org/2000/svg'
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns:usvg='https://github.com/RazrFalcon/usvg'
    usvg:version='0.5.0'>
    <defs/>
    <g
        id='some_group'>
        <path
            d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
    </g>
</svg>
");

test!(simplify_paths_1, false,
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
    <path d='M 10 20 L 10 30 Z Z Z'/>
</svg>",
"<svg
    xmlns='http://www.w3.org/2000/svg'
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns:usvg='https://github.com/RazrFalcon/usvg'
    usvg:version='0.5.0'>
    <defs/>
    <path
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
    xmlns='http://www.w3.org/2000/svg'
    width='1'
    height='1'
    viewBox='0 0 1 1'
    xmlns:usvg='https://github.com/RazrFalcon/usvg'
    usvg:version='0.5.0'>
    <defs/>
    <path
        d='M 10 20 L 10 30'/>
    <path
        d='M 10 20 L 10 30'/>
</svg>
");

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
//    xmlns:usvg='https://github.com/RazrFalcon/usvg'
//    usvg:version='0.5.0'>
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
