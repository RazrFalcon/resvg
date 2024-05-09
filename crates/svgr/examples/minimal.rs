use usvgr::fontdb;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Usage:\n\tminimal <in-svg> <out-png>");
        return;
    }

    let tree = {
        let mut opt = usvgr::Options::default();
        // Get file's absolute directory.
        opt.resources_dir = std::fs::canonicalize(&args[1])
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()));

        let mut fontdb = fontdb::Database::new();
        fontdb.load_system_fonts();

        let svg_data = std::fs::read(&args[1]).unwrap();
        usvgr::Tree::from_data(&svg_data, &opt, &fontdb).unwrap()
    };

    let pixmap_size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
    let mut cache = svgr::SvgrCache::none();
    let ctx = svgr::Context::new_from_pixmap(&pixmap);
    svgr::render(
        &tree,
        tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
        &mut cache,
        &ctx,
    );
    pixmap.save_png(&args[2]).unwrap();
}
