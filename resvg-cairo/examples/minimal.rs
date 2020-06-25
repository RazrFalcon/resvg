fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Usage:\n\tminimal <in-svg> <out-png>");
        return;
    }

    let mut opt = usvg::Options::default();
    opt.path = Some(args[1].clone().into());

    let rtree = usvg::Tree::from_file(&args[1], &opt).unwrap();
    let img = resvg_cairo::render_to_image(&rtree, &opt, usvg::FitTo::Original, None).unwrap();
    let mut file = std::fs::File::create(&args[2]).unwrap();
    img.write_to_png(&mut file).unwrap();
}
