fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Usage:\n\tminimal <in-svg> <out-png>");
        return;
    }

    let mut opt = usvg::Options::default();
    opt.path = Some(args[1].clone().into());

    let rtree = usvg::Tree::from_file(&args[1], &opt).unwrap();
    let img = resvg_qt::render_to_image(&rtree, usvg::FitTo::Original, None).unwrap();
    img.save(&args[2]);
}
