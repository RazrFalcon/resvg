extern crate resvg;

use std::env;
use std::path::Path;

// TODO: write doc

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage:\n\tminimal <in-svg> <out-png>");
        return;
    }

    let _resvg = resvg::init();

    let opt = resvg::Options {
        path: Some(args[1].clone().into()),
        .. resvg::Options::default()
    };

    let rtree = resvg::parse_rtree_from_file(&args[1], &opt).unwrap();
    let backend = resvg::default_backend();
    let img = backend.render_to_image(&rtree, &opt).unwrap();
    img.save(Path::new(&args[2]));
}
