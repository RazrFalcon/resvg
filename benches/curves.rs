use bencher::Bencher;

// We are using `text` instead of a `path` since it's easier to create a lot of curves this way.
// From the text below, usvg will create ~400KiB SVG file.

macro_rules! bench_backend {
    ($name:ident, $backend:ident, $input:expr) => {
        fn $name(bencher: &mut Bencher) {
            let tree = usvg::Tree::from_str(&$input, &usvg::Options::default()).unwrap();
            bencher.iter(|| {
                let _ = bencher::black_box($backend::render_to_image(&tree, &$backend::Options::default()));
            })
        }
    };
}

const BASE_SVG: &str = "\
<svg viewBox='0 0 1000 300' xmlns='http://www.w3.org/2000/svg' font-size='32' #style#>
    <text x='10' y='40' font-family='Arial'>AaBbCcDdEeFfGgHhIiJjKkLlMmNnOoPpQqRrSsTtUuVvWwXxYyZz</text>
    <text x='10' y='80' font-family='Times New Roman'>AaBbCcDdEeFfGgHhIiJjKkLlMmNnOoPpQqRrSsTtUuVvWwXxYyZz</text>
    <text x='10' y='120' font-family='Noto Sans Arabic'>
        <tspan x='10' dy='40'>اللُّغَة العَرَبِيّة هي أكثر اللغات تحدثاً ونطقاً ضمن مجموعة اللغات السامية،</tspan>
        <tspan x='10'>وإحدى أكثر اللغات انتشاراً في العالم، يتحدثها أكثر من 467 مليون نسمة،</tspan>
    </text>
    <text y='200' font-family='Droid Sans Fallback' font-size='28'>
        <tspan x='10'>気候は四季の変化に富み、国土の多くは山地で、人口は平野部に集中している。</tspan>
        <tspan x='10' dy='40'>国内には行政区分として47の都道府県があり、日本人・琉球民族・アイヌ</tspan>
        <tspan x='10' dy='40'>・外国人系の人々などが居住し、事実上の公用語として日本語が使用される</tspan>
    </text>
</svg>";

bench_backend!(stroke_cairo, resvg_cairo,    BASE_SVG.replace("#style#", "fill='none' stroke='black'"));
bench_backend!(stroke_qt, resvg_qt,          BASE_SVG.replace("#style#", "fill='none' stroke='black'"));
bench_backend!(stroke_raqote, resvg_raqote,  BASE_SVG.replace("#style#", "fill='none' stroke='black'"));
bench_backend!(stroke_skia, resvg_skia,      BASE_SVG.replace("#style#", "fill='none' stroke='black'"));

bench_backend!(fill_cairo, resvg_cairo,    BASE_SVG.replace("#style#", "fill='black'"));
bench_backend!(fill_qt, resvg_qt,          BASE_SVG.replace("#style#", "fill='black'"));
bench_backend!(fill_raqote, resvg_raqote,  BASE_SVG.replace("#style#", "fill='black'"));
bench_backend!(fill_skia, resvg_skia,      BASE_SVG.replace("#style#", "fill='black'"));

bench_backend!(fill_and_stroke_cairo, resvg_cairo,    BASE_SVG.replace("#style#", "fill='green' stroke='black'"));
bench_backend!(fill_and_stroke_qt, resvg_qt,          BASE_SVG.replace("#style#", "fill='green' stroke='black'"));
bench_backend!(fill_and_stroke_raqote, resvg_raqote,  BASE_SVG.replace("#style#", "fill='green' stroke='black'"));
bench_backend!(fill_and_stroke_skia, resvg_skia,      BASE_SVG.replace("#style#", "fill='green' stroke='black'"));

bench_backend!(dashed_stroke_cairo, resvg_cairo,    BASE_SVG.replace("#style#", "fill='none' stroke='black' stroke-dasharray='2 4 6'"));
bench_backend!(dashed_stroke_qt, resvg_qt,          BASE_SVG.replace("#style#", "fill='none' stroke='black' stroke-dasharray='2 4 6'"));
bench_backend!(dashed_stroke_raqote, resvg_raqote,  BASE_SVG.replace("#style#", "fill='none' stroke='black' stroke-dasharray='2 4 6'"));
bench_backend!(dashed_stroke_skia, resvg_skia,      BASE_SVG.replace("#style#", "fill='none' stroke='black' stroke-dasharray='2 4 6'"));

const GRADIENTS_SVG: &str = "\
<svg viewBox='0 0 1000 300' xmlns='http://www.w3.org/2000/svg' font-size='32'>
    <linearGradient id='lg1'>
        <stop offset='0' stop-color='green'/>
        <stop offset='1' stop-color='orange'/>
    </linearGradient>
    <linearGradient id='lg2'>
        <stop offset='0' stop-color='orange'/>
        <stop offset='1' stop-color='green'/>
    </linearGradient>
    <g fill='url(#lg1)' stroke='url(#lg2)'>
        <text x='10' y='40' font-family='Arial'>AaBbCcDdEeFfGgHhIiJjKkLlMmNnOoPpQqRrSsTtUuVvWwXxYyZz</text>
        <text x='10' y='80' font-family='Times New Roman'>AaBbCcDdEeFfGgHhIiJjKkLlMmNnOoPpQqRrSsTtUuVvWwXxYyZz</text>
        <text x='10' y='120' font-family='Noto Sans Arabic'>
            <tspan x='10' dy='40'>اللُّغَة العَرَبِيّة هي أكثر اللغات تحدثاً ونطقاً ضمن مجموعة اللغات السامية،</tspan>
            <tspan x='10'>وإحدى أكثر اللغات انتشاراً في العالم، يتحدثها أكثر من 467 مليون نسمة،</tspan>
        </text>
        <text y='200' font-family='Droid Sans Fallback' font-size='28'>
            <tspan x='10'>気候は四季の変化に富み、国土の多くは山地で、人口は平野部に集中している。</tspan>
            <tspan x='10' dy='40'>国内には行政区分として47の都道府県があり、日本人・琉球民族・アイヌ</tspan>
            <tspan x='10' dy='40'>・外国人系の人々などが居住し、事実上の公用語として日本語が使用される</tspan>
        </text>
    </g>
</svg>";

bench_backend!(fill_and_stroke_with_gradient_cairo, resvg_cairo,    GRADIENTS_SVG);
bench_backend!(fill_and_stroke_with_gradient_qt, resvg_qt,          GRADIENTS_SVG);
bench_backend!(fill_and_stroke_with_gradient_raqote, resvg_raqote,  GRADIENTS_SVG);
bench_backend!(fill_and_stroke_with_gradient_skia, resvg_skia,      GRADIENTS_SVG);

const PATTERN_SVG: &str = "\
<svg viewBox='0 0 1000 300' xmlns='http://www.w3.org/2000/svg' font-size='32'>
    <pattern id='patt1' patternUnits='userSpaceOnUse' width='2' height='2'>
        <rect x='0' y='0' width='1' height='1' fill='grey'/>
        <rect x='1' y='1' width='1' height='1' fill='green'/>
    </pattern>
    <g fill='url(#patt1)' stroke='url(#patt1)'>
        <text x='10' y='40' font-family='Arial'>AaBbCcDdEeFfGgHhIiJjKkLlMmNnOoPpQqRrSsTtUuVvWwXxYyZz</text>
        <text x='10' y='80' font-family='Times New Roman'>AaBbCcDdEeFfGgHhIiJjKkLlMmNnOoPpQqRrSsTtUuVvWwXxYyZz</text>
        <text x='10' y='120' font-family='Noto Sans Arabic'>
            <tspan x='10' dy='40'>اللُّغَة العَرَبِيّة هي أكثر اللغات تحدثاً ونطقاً ضمن مجموعة اللغات السامية،</tspan>
            <tspan x='10'>وإحدى أكثر اللغات انتشاراً في العالم، يتحدثها أكثر من 467 مليون نسمة،</tspan>
        </text>
        <text y='200' font-family='Droid Sans Fallback' font-size='28'>
            <tspan x='10'>気候は四季の変化に富み、国土の多くは山地で、人口は平野部に集中している。</tspan>
            <tspan x='10' dy='40'>国内には行政区分として47の都道府県があり、日本人・琉球民族・アイヌ</tspan>
            <tspan x='10' dy='40'>・外国人系の人々などが居住し、事実上の公用語として日本語が使用される</tspan>
        </text>
    </g>
</svg>";

bench_backend!(fill_and_stroke_with_pattern_cairo, resvg_cairo,    PATTERN_SVG);
bench_backend!(fill_and_stroke_with_pattern_qt, resvg_qt,          PATTERN_SVG);
bench_backend!(fill_and_stroke_with_pattern_raqote, resvg_raqote,  PATTERN_SVG);
bench_backend!(fill_and_stroke_with_pattern_skia, resvg_skia,      PATTERN_SVG);

const FILL_CIRCLE_SVG: &str = "\
<svg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'>
    <circle cx='100' cy='100' r='80' fill='green'/>
</svg>";

bench_backend!(fill_circle_cairo, resvg_cairo,    FILL_CIRCLE_SVG);
bench_backend!(fill_circle_qt, resvg_qt,          FILL_CIRCLE_SVG);
bench_backend!(fill_circle_raqote, resvg_raqote,  FILL_CIRCLE_SVG);
bench_backend!(fill_circle_skia, resvg_skia,      FILL_CIRCLE_SVG);

const STROKE_CIRCLE_SVG: &str = "\
<svg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'>
    <circle cx='100' cy='100' r='80' fill='none' stroke='green'/>
</svg>";

bench_backend!(stroke_circle_cairo, resvg_cairo,    STROKE_CIRCLE_SVG);
bench_backend!(stroke_circle_qt, resvg_qt,          STROKE_CIRCLE_SVG);
bench_backend!(stroke_circle_raqote, resvg_raqote,  STROKE_CIRCLE_SVG);
bench_backend!(stroke_circle_skia, resvg_skia,      STROKE_CIRCLE_SVG);

const GRADIENT_CIRCLE_SVG: &str = "\
<svg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'>
    <linearGradient id='lg1'>
        <stop offset='0' stop-color='green'/>
        <stop offset='1' stop-color='orange'/>
    </linearGradient>
    <linearGradient id='lg2'>
        <stop offset='0' stop-color='orange'/>
        <stop offset='1' stop-color='green'/>
    </linearGradient>
    <circle cx='100' cy='100' r='80' fill='url(#lg1)' stroke='url(#lg2)'/>
</svg>";

bench_backend!(fill_and_stroke_circle_with_gradient_cairo, resvg_cairo,    GRADIENT_CIRCLE_SVG);
bench_backend!(fill_and_stroke_circle_with_gradient_qt, resvg_qt,          GRADIENT_CIRCLE_SVG);
bench_backend!(fill_and_stroke_circle_with_gradient_raqote, resvg_raqote,  GRADIENT_CIRCLE_SVG);
bench_backend!(fill_and_stroke_circle_with_gradient_skia, resvg_skia,      GRADIENT_CIRCLE_SVG);

const PATTERN_CIRCLE_SVG: &str = "\
<svg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'>
    <pattern id='patt1' patternUnits='userSpaceOnUse' width='20' height='20'>
        <rect x='0' y='0' width='10' height='10' fill='grey'/>
        <rect x='10' y='10' width='10' height='10' fill='green'/>
    </pattern>
    <circle cx='100' cy='100' r='80' fill='url(#patt1)' stroke='url(#patt1)'/>
</svg>";

bench_backend!(fill_and_stroke_circle_with_pattern_cairo, resvg_cairo,    PATTERN_CIRCLE_SVG);
bench_backend!(fill_and_stroke_circle_with_pattern_qt, resvg_qt,          PATTERN_CIRCLE_SVG);
bench_backend!(fill_and_stroke_circle_with_pattern_raqote, resvg_raqote,  PATTERN_CIRCLE_SVG);
bench_backend!(fill_and_stroke_circle_with_pattern_skia, resvg_skia,      PATTERN_CIRCLE_SVG);

bencher::benchmark_group!(
    benches,
    stroke_cairo,
    stroke_qt,
    stroke_raqote,
    stroke_skia,
    fill_cairo,
    fill_qt,
    fill_raqote,
    fill_skia,
    fill_and_stroke_cairo,
    fill_and_stroke_qt,
    fill_and_stroke_raqote,
    fill_and_stroke_skia,
    dashed_stroke_cairo,
    dashed_stroke_qt,
    dashed_stroke_raqote,
    dashed_stroke_skia,
    fill_and_stroke_with_gradient_cairo,
    fill_and_stroke_with_gradient_qt,
    fill_and_stroke_with_gradient_raqote,
    fill_and_stroke_with_gradient_skia,
    fill_and_stroke_with_pattern_cairo,
    fill_and_stroke_with_pattern_qt,
    fill_and_stroke_with_pattern_raqote,
    fill_and_stroke_with_pattern_skia,
    fill_circle_cairo,
    fill_circle_qt,
    fill_circle_raqote,
    fill_circle_skia,
    stroke_circle_cairo,
    stroke_circle_qt,
    stroke_circle_raqote,
    stroke_circle_skia,
    fill_and_stroke_circle_with_gradient_cairo,
    fill_and_stroke_circle_with_gradient_qt,
    fill_and_stroke_circle_with_gradient_raqote,
    fill_and_stroke_circle_with_gradient_skia,
    fill_and_stroke_circle_with_pattern_cairo,
    fill_and_stroke_circle_with_pattern_qt,
    fill_and_stroke_circle_with_pattern_raqote,
    fill_and_stroke_circle_with_pattern_skia
);

bencher::benchmark_main!(benches);
