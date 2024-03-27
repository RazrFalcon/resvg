use bencher::{benchmark_group, benchmark_main, Bencher};

fn path_large(bencher: &mut Bencher) {
    let text = std::fs::read_to_string("path-large.txt").unwrap();
    bencher.iter(|| {
        for t in svgrtypes::PathParser::from(text.as_str()) {
            let _ = t.unwrap();
        }
    })
}

benchmark_group!(paths, path_large);
benchmark_main!(paths);
