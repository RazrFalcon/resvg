use usvg;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Usage:\n\tusvg_converter <in-svg> <out-svg>");
        return;
    }
    let mut opt = usvg::Options::default();
    // Get file's absolute directory.
    opt.resources_dir = std::fs::canonicalize(&args[1]).ok().and_then(|p| p.parent().map(|p| p.to_path_buf()));
    opt.fontdb.load_system_fonts();

    let svg_data = std::fs::read(&args[1]).unwrap();
    let rtree = usvg::Tree::from_data(&svg_data, &opt.to_ref()).unwrap();
    std::fs::write(&args[2], rtree.to_string(&usvg::XmlOptions::default())).unwrap();
}
