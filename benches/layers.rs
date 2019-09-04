use bencher::Bencher;
use resvg::usvg;

macro_rules! bench_backend {
    ($name:ident, $backend:ident, $input:expr) => {
        fn $name(bencher: &mut Bencher) {
            let tree = usvg::Tree::from_str($input, &usvg::Options::default()).unwrap();
            bencher.iter(|| {
                let _ = bencher::black_box(resvg::$backend::render_to_image(&tree, &resvg::Options::default()));
            })
        }
    };
}

const ELEMENT_WITH_OPACITY_SVG: &str = "\
<svg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'>
    <rect x='40' y='40' width='160' height='160' fill='blue' opacity='0.5'/>
</svg>";

bench_backend!(element_with_opacity_cairo, backend_cairo, ELEMENT_WITH_OPACITY_SVG);
bench_backend!(element_with_opacity_qt, backend_qt, ELEMENT_WITH_OPACITY_SVG);
bench_backend!(element_with_opacity_raqote, backend_raqote, ELEMENT_WITH_OPACITY_SVG);
bench_backend!(element_with_opacity_skia, backend_skia, ELEMENT_WITH_OPACITY_SVG);

const GROUPS_WITH_OPACITY_SVG: &str = "\
<svg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'>
    <g opacity='0.5'>
        <rect x='20' y='20' width='160' height='160' fill='green'/>
        <g opacity='0.5'>
            <rect x='40' y='40' width='160' height='160' fill='blue'/>
        </g>
    </g>
</svg>";

bench_backend!(groups_with_opacity_cairo, backend_cairo, GROUPS_WITH_OPACITY_SVG);
bench_backend!(groups_with_opacity_qt, backend_qt, GROUPS_WITH_OPACITY_SVG);
bench_backend!(groups_with_opacity_raqote, backend_raqote, GROUPS_WITH_OPACITY_SVG);
bench_backend!(groups_with_opacity_skia, backend_skia, GROUPS_WITH_OPACITY_SVG);

const CLIP_PATH_SVG: &str = "\
<svg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'>
    <clipPath id='clip1'>
        <path d='M 100 15 l 50 160 l -130 -100 l 160 0 l -130 100 z'/>
    </clipPath>
    <rect x='0' y='0' width='200' height='200' fill='green' clip-path='url(#clip1)'/>
</svg>";

bench_backend!(clip_path_cairo, backend_cairo, CLIP_PATH_SVG);
bench_backend!(clip_path_qt, backend_qt, CLIP_PATH_SVG);
bench_backend!(clip_path_raqote, backend_raqote, CLIP_PATH_SVG);
bench_backend!(clip_path_skia, backend_skia, CLIP_PATH_SVG);

const NESTED_CLIP_PATH_SVG: &str = "\
<svg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'>
    <clipPath id='clip1' clip-path='url(#clip2)'>
        <path d='M 100 15 l 50 160 l -130 -100 l 160 0 l -130 100 z' clip-rule='evenodd'/>
    </clipPath>
    <clipPath id='clip2'>
        <circle x='100' cy='100' r='60'/>
    </clipPath>
    <rect x='0' y='0' width='200' height='200' fill='green' clip-path='url(#clip1)'/>
</svg>";

bench_backend!(nested_clip_path_cairo, backend_cairo, NESTED_CLIP_PATH_SVG);
bench_backend!(nested_clip_path_qt, backend_qt, NESTED_CLIP_PATH_SVG);
bench_backend!(nested_clip_path_raqote, backend_raqote, NESTED_CLIP_PATH_SVG);
bench_backend!(nested_clip_path_skia, backend_skia, NESTED_CLIP_PATH_SVG);

const MASK_SVG: &str = "\
<svg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'>
    <linearGradient id='lg1'>
        <stop offset='0' stop-color='white' stop-opacity='0'/>
        <stop offset='1' stop-color='black'/>
    </linearGradient>
    <mask id='mask1'>
        <rect x='20' y='20' width='160' height='160' fill='url(#lg1)'/>
    </mask>
    <rect x='0' y='0' width='200' height='200' fill='green' mask='url(#mask1)'/>
</svg>";

bench_backend!(mask_cairo, backend_cairo, MASK_SVG);
bench_backend!(mask_qt, backend_qt, MASK_SVG);
bench_backend!(mask_raqote, backend_raqote, MASK_SVG);
bench_backend!(mask_skia, backend_skia, MASK_SVG);

bencher::benchmark_group!(
    benches,
    element_with_opacity_cairo,
    element_with_opacity_qt,
    element_with_opacity_raqote,
    element_with_opacity_skia,
    groups_with_opacity_cairo,
    groups_with_opacity_qt,
    groups_with_opacity_raqote,
    groups_with_opacity_skia,
    clip_path_cairo,
    clip_path_qt,
    clip_path_raqote,
    clip_path_skia,
    nested_clip_path_cairo,
    nested_clip_path_qt,
    nested_clip_path_raqote,
    nested_clip_path_skia,
    mask_cairo,
    mask_qt,
    mask_raqote,
    mask_skia
);

bencher::benchmark_main!(benches);
