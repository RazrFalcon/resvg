extern crate resvg;

use std::env;
use std::path::Path;

use resvg::prelude::*;

// TODO: write doc

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!("Usage:\n\tminimal <in-svg> <out-png>");
        return;
    }

    let mut opt = resvg::Options::default();
    opt.usvg.path = Some(args[1].clone().into());

    let rtree = usvg::Tree::from_file(&args[1], &opt.usvg).unwrap();
    let backend = resvg::default_backend();
    let img = backend.render_to_image(&rtree, &opt).unwrap();
    img.save(Path::new(&args[2]));
}
