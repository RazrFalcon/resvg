extern crate resvg;
extern crate svgdom;

use resvg::tree::prelude::*;

use svgdom::ToStringWithOptions;

// Unlike in https://github.com/RazrFalcon/resvg-test-suite
// here we test preprocessor and resulting rtree structure,
// which can't be tested by comparing rendered images.

macro_rules! assert_eq_text {
    ($left:expr, $right:expr) => ({
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    panic!("assertion failed: `(left == right)` \
                           \nleft:  `{}`\nright: `{}`",
                           left_val, right_val)
                }
            }
        }
    })
}

static SVG_ATTRS: &str = "xmlns:xlink='http://www.w3.org/1999/xlink' xmlns='http://www.w3.org/2000/svg' \
xmlns:resvg='https://github.com/RazrFalcon/resvg' resvg:version='0.1.0'";

macro_rules! test {
    ($name:ident, $input:expr, $output:expr, $keep_named_groups:expr) => {
        #[test]
        fn $name() {
            let output = $output.replace("SVG_ATTRS", SVG_ATTRS);

            let re_opt = resvg::Options {
                keep_named_groups: $keep_named_groups,
                .. resvg::Options::default()
            };
            let rtree = resvg::parse_rtree_from_data($input, &re_opt).unwrap();

            let dom_opt = svgdom::WriteOptions {
                use_single_quote: true,
                attributes_order: svgdom::AttributesOrder::Specification,
                .. svgdom::WriteOptions::default()
            };

            assert_eq_text!(rtree.to_svgdom().to_string_with_opt(&dom_opt), output);
        }
    };
}

test!(minimal,
"<svg viewBox='0 0 1 1'>
    <rect width='10' height='10'/>
</svg>",
"<svg width='1' height='1' viewBox='0 0 1 1' preserveAspectRatio='xMidYMid' SVG_ATTRS>
    <defs/>
    <path fill='#000000' stroke='none' d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
</svg>
", false);

test!(groups,
"<svg viewBox='0 0 1 1'>
    <g>
        <g>
            <rect width='10' height='10'/>
        </g>
    </g>
</svg>",
"<svg width='1' height='1' viewBox='0 0 1 1' preserveAspectRatio='xMidYMid' SVG_ATTRS>
    <defs/>
    <path fill='#000000' stroke='none' d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
</svg>
", false);

test!(keep_groups_with_opacity,
"<svg viewBox='0 0 1 1'>
    <g opacity='0.5'>
        <g>
            <rect width='10' height='10'/>
        </g>
    </g>
</svg>",
"<svg width='1' height='1' viewBox='0 0 1 1' preserveAspectRatio='xMidYMid' SVG_ATTRS>
    <defs/>
    <g opacity='0.5'>
        <path fill='#000000' stroke='none' d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
    </g>
</svg>
", false);

test!(ignore_groups_with_id,
"<svg viewBox='0 0 1 1'>
    <g id='some_group'>
        <rect width='10' height='10'/>
    </g>
</svg>",
"<svg width='1' height='1' viewBox='0 0 1 1' preserveAspectRatio='xMidYMid' SVG_ATTRS>
    <defs/>
    <path fill='#000000' stroke='none' d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
</svg>
", false);

// Note that we use `true` flag.
test!(keep_groups_with_id,
"<svg viewBox='0 0 1 1'>
    <g id='some_group'>
        <rect width='10' height='10'/>
    </g>
</svg>",
"<svg width='1' height='1' viewBox='0 0 1 1' preserveAspectRatio='xMidYMid' SVG_ATTRS>
    <defs/>
    <g id='some_group'>
        <path fill='#000000' stroke='none' d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
    </g>
</svg>
", true);

// No need to keep empty groups even if `keep_named_groups` is enabled.
test!(ignore_empty_groups_with_id,
"<svg viewBox='0 0 1 1'>
    <g id='some_group'/>
    <rect width='10' height='10'/>
</svg>",
"<svg width='1' height='1' viewBox='0 0 1 1' preserveAspectRatio='xMidYMid' SVG_ATTRS>
    <defs/>
    <path fill='#000000' stroke='none' d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
</svg>
", true);

// All supported elements should be listed.
//
// We keep id's even if `keep_named_groups` is disabled.
//
// ID on `svg`, `defs`, `stop` and `tspan` is ignored because they can't be rendered directly.
test!(preserve_ids,
"<svg id='svg1' viewBox='0 0 1 1'>
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
    <text id='text1'>Some text</text>
    <text id='text2'><tspan id='tspan2'>Some text</tspan></text>
    <image id='image1' width='1' height='1' xlink:href='data:image/png;base64,
        iVBORw0KGgoAAAANSUhEUgAAABAAAAAQAQMAAAAlPW0iAAAAB3RJTUUH4gMLDwAjrsLbtwAAAAlw
        SFlzAAAuIwAALiMBeKU/dgAAABl0RVh0Q29tbWVudABDcmVhdGVkIHdpdGggR0lNUFeBDhcAAAAG
        UExURQAA/xjQP14JpdQAAAABYktHRACIBR1IAAAAFklEQVR42mMAgvp/IJTAhgdB1ADVAgDvdAnx
        N1Ib1gAAAABJRU5ErkJggg=='/>
</svg>",
"<svg width='1' height='1' viewBox='0 0 1 1' preserveAspectRatio='xMidYMid' SVG_ATTRS>
    <defs>
        <linearGradient id='lg1' x1='0' y1='0' x2='1' y2='0' gradientUnits='objectBoundingBox' spreadMethod='pad'>
            <stop stop-color='#ffffff' stop-opacity='1' offset='0'/>
            <stop stop-color='#000000' stop-opacity='1' offset='1'/>
        </linearGradient>
        <radialGradient id='rg1' cx='0.5' cy='0.5' r='0.5' fx='0.5' fy='0.5' gradientUnits='objectBoundingBox' spreadMethod='pad'>
            <stop stop-color='#ffffff' stop-opacity='1' offset='0'/>
            <stop stop-color='#000000' stop-opacity='1' offset='1'/>
        </radialGradient>
        <clipPath id='clip1' clipPathUnits='userSpaceOnUse'>
            <path id='rect2' fill='#000000' stroke='none' d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
        </clipPath>
        <pattern id='patt1' x='0' y='0' width='1' height='1' patternUnits='objectBoundingBox' patternContentUnits='userSpaceOnUse'>
            <path fill='#000000' stroke='none' d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
        </pattern>
    </defs>
    <g clip-path='url(#clip1)'>
        <path id='rect1' fill='url(#lg1)' stroke='url(#rg1)' d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
    </g>
    <path id='path1' fill='url(#patt1)' stroke='none' d='M 10 20 L 30 40'/>
    <text id='text1'>
        <tspan x='0' y='0'>
            <tspan fill='#000000' stroke='none' font-family='Times New Roman' font-size='12'>Some text</tspan>
        </tspan>
    </text>
    <text id='text2'>
        <tspan x='0' y='0'>
            <tspan fill='#000000' stroke='none' font-family='Times New Roman' font-size='12'>Some text</tspan>
        </tspan>
    </text>
    <image id='image1' preserveAspectRatio='xMidYMid' x='0' y='0' width='1' height='1' xlink:href='data:image/png;base64,
iVBORw0KGgoAAAANSUhEUgAAABAAAAAQAQMAAAAlPW0iAAAAB3RJTUUH4gMLDwAj
rsLbtwAAAAlwSFlzAAAuIwAALiMBeKU/dgAAABl0RVh0Q29tbWVudABDcmVhdGVk
IHdpdGggR0lNUFeBDhcAAAAGUExURQAA/xjQP14JpdQAAAABYktHRACIBR1IAAAA
FklEQVR42mMAgvp/IJTAhgdB1ADVAgDvdAnxN1Ib1gAAAABJRU5ErkJggg=='/>
</svg>
", false);

// clipPath requires a new canvas so we have to indicate this by adding a new group.
test!(group_clip_path,
"<svg viewBox='0 0 1 1'>
    <clipPath id='clip1'>
        <rect width='10' height='10'/>
    </clipPath>
    <rect clip-path='url(#clip1)' width='10' height='10'/>
</svg>",
"<svg width='1' height='1' viewBox='0 0 1 1' preserveAspectRatio='xMidYMid' SVG_ATTRS>
    <defs>
        <clipPath id='clip1' clipPathUnits='userSpaceOnUse'>
            <path fill='#000000' stroke='none' d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
        </clipPath>
    </defs>
    <g clip-path='url(#clip1)'>
        <path fill='#000000' stroke='none' d='M 0 0 L 10 0 L 10 10 L 0 10 Z'/>
    </g>
</svg>
", false);
