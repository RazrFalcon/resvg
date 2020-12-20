use usvg::SystemFontDB;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Usage:\n\tminimal <in-svg> <out-png>");
        return;
    }

    let mut opt = usvg::Options::default();
    opt.path = Some(args[1].clone().into());
    opt.fontdb.load_system_fonts();
    opt.fontdb.set_generic_families();

    let rtree = usvg::Tree::from_file(&args[1], &opt).unwrap();

    let pixmap_size = rtree.svg_node().size.to_screen_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
    resvg::render(&rtree, usvg::FitTo::Original, pixmap.as_mut()).unwrap();
    pixmap.save_png(&args[2]).unwrap();
}
