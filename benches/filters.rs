use bencher::Bencher;

macro_rules! bench_backend {
    ($name:ident, $backend:ident, $input:expr) => {
        fn $name(bencher: &mut Bencher) {
            let tree = usvg::Tree::from_str($input, &usvg::Options::default()).unwrap();
            bencher.iter(|| {
                let _ = bencher::black_box($backend::render_to_image(&tree, &$backend::Options::default()));
            })
        }
    };
}

const BLEND_MULTIPLY_SVG: &str = "\
<svg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'>
    <filter id='filter1'>
        <feFlood flood-color='lightblue'/>
        <feBlend mode='multiply' in2='SourceGraphic'/>
    </filter>
    <rect x='20' y='20' width='160' height='160' fill='green' filter='url(#filter1)'/>
</svg>";

bench_backend!(blend_multiply_cairo, resvg_cairo, BLEND_MULTIPLY_SVG);
bench_backend!(blend_multiply_qt, resvg_qt, BLEND_MULTIPLY_SVG);
bench_backend!(blend_multiply_raqote, resvg_raqote, BLEND_MULTIPLY_SVG);
bench_backend!(blend_multiply_skia, resvg_skia, BLEND_MULTIPLY_SVG);

// fn box_blur_100px(bencher: &mut Bencher) {
//     let tree = usvg::Tree::from_str("
//         <svg viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg'>
//             <filter id='filter1'>
//                 <feGaussianBlur stdDeviation='4'/>
//             </filter>
//             <circle cx='50' cy='50' r='40' fill='green' stroke='red' filter='url(#filter1)'/>
//         </svg>", &usvg::Options::default()).unwrap();

//     bencher.iter(|| {
//         let _ = bencher::black_box(resvg::default_backend().render_to_image(&tree, &resvg::Options::default()));
//     })
// }

// fn iir_blur_100px(bencher: &mut Bencher) {
//     let tree = usvg::Tree::from_str("
//         <svg viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg'>
//             <filter id='filter1'>
//                 <feGaussianBlur stdDeviation='1'/>
//             </filter>
//             <circle cx='50' cy='50' r='40' fill='green' stroke='red' filter='url(#filter1)'/>
//         </svg>", &usvg::Options::default()).unwrap();

//     bencher.iter(|| {
//         let _ = bencher::black_box(resvg::default_backend().render_to_image(&tree, &resvg::Options::default()));
//     })
// }

// fn box_blur_500px(bencher: &mut Bencher) {
//     let tree = usvg::Tree::from_str("
//         <svg viewBox='0 0 500 500' xmlns='http://www.w3.org/2000/svg'>
//             <filter id='filter1'>
//                 <feGaussianBlur stdDeviation='4'/>
//             </filter>
//             <circle cx='250' cy='250' r='200' fill='green' stroke='red' filter='url(#filter1)'/>
//         </svg>", &usvg::Options::default()).unwrap();

//     bencher.iter(|| {
//         let _ = bencher::black_box(resvg::default_backend().render_to_image(&tree, &resvg::Options::default()));
//     })
// }

// fn iir_blur_500px(bencher: &mut Bencher) {
//     let tree = usvg::Tree::from_str("
//         <svg viewBox='0 0 500 500' xmlns='http://www.w3.org/2000/svg'>
//             <filter id='filter1'>
//                 <feGaussianBlur stdDeviation='1'/>
//             </filter>
//             <circle cx='250' cy='250' r='200' fill='green' stroke='red' filter='url(#filter1)'/>
//         </svg>", &usvg::Options::default()).unwrap();

//     bencher.iter(|| {
//         let _ = bencher::black_box(resvg::default_backend().render_to_image(&tree, &resvg::Options::default()));
//     })
// }

const COMPOSITE_OVER_SVG: &str = "\
<svg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'>
    <filter id='filter1'>
        <feFlood flood-color='lightblue' x='50' y='50' width='100' height='100'/>
        <feComposite operator='over' in2='SourceGraphic'/>
    </filter>
    <rect x='20' y='20' width='160' height='160' fill='green' filter='url(#filter1)'/>
</svg>";

bench_backend!(composite_over_cairo, resvg_cairo, COMPOSITE_OVER_SVG);
bench_backend!(composite_over_qt, resvg_qt, COMPOSITE_OVER_SVG);
bench_backend!(composite_over_raqote, resvg_raqote, COMPOSITE_OVER_SVG);
bench_backend!(composite_over_skia, resvg_skia, COMPOSITE_OVER_SVG);

const COMPOSITE_ARITHMETIC_SVG: &str = "\
<svg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'>
    <filter id='filter1'>
        <feFlood flood-color='blue' flood-opacity='0.8'/>
        <feComposite operator='arithmetic' in2='SourceGraphic' k1='0.1' k2='0.2' k3='0.3' k4='0.4'/>
    </filter>
    <rect x='20' y='20' width='160' height='160' fill='green' filter='url(#filter1)'/>
</svg>";

bench_backend!(composite_arithmetic_cairo, resvg_cairo, COMPOSITE_ARITHMETIC_SVG);
bench_backend!(composite_arithmetic_qt, resvg_qt, COMPOSITE_ARITHMETIC_SVG);
bench_backend!(composite_arithmetic_raqote, resvg_raqote, COMPOSITE_ARITHMETIC_SVG);
bench_backend!(composite_arithmetic_skia, resvg_skia, COMPOSITE_ARITHMETIC_SVG);

const COLOR_MATRIX_SVG: &str = "\
<svg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'>
    <linearGradient id='lg1'>
        <stop offset='0' stop-color='#cc00cc'/>
        <stop offset='.33' stop-color='#228822'/>
        <stop offset='.67' stop-color='#400000'/>
        <stop offset='1' stop-color='#a0a0ff'/>
    </linearGradient>
    <filter id='filter1'>
        <feColorMatrix type='matrix' values='
            0.3 0.3 0.3 0 0
            0.3 0.3 0.3 0 0
            0.3 0.3 0.3 0 0
            0.3 0.3 0.3 0 0'/>
    </filter>
    <rect x='20' y='20' width='160' height='160' fill='url(#lg1)' filter='url(#filter1)'/>
</svg>";

bench_backend!(color_matrix_cairo, resvg_cairo, COLOR_MATRIX_SVG);
bench_backend!(color_matrix_qt, resvg_qt, COLOR_MATRIX_SVG);
bench_backend!(color_matrix_raqote, resvg_raqote, COLOR_MATRIX_SVG);
bench_backend!(color_matrix_skia, resvg_skia, COLOR_MATRIX_SVG);

bencher::benchmark_group!(
    blend,
    blend_multiply_cairo,
    blend_multiply_qt,
    blend_multiply_raqote,
    blend_multiply_skia
);

// bencher::benchmark_group!(
//     blur,
//     box_blur_100px,
//     iir_blur_100px,
//     box_blur_500px,
//     iir_blur_500px
// );

bencher::benchmark_group!(
    composite,
    composite_over_cairo,
    composite_over_qt,
    composite_over_raqote,
    composite_over_skia,
    composite_arithmetic_cairo,
    composite_arithmetic_qt,
    composite_arithmetic_raqote,
    composite_arithmetic_skia
);

bencher::benchmark_group!(
    color_matrix,
    color_matrix_cairo,
    color_matrix_qt,
    color_matrix_raqote,
    color_matrix_skia
);

bencher::benchmark_main!(blend, composite, color_matrix);
