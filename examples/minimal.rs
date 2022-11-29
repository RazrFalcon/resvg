use usvg_text_layout::{fontdb, TreeTextToPath};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        println!("Usage:\n\tminimal <in-svg> <out-png>");
        return;
    }

    let mut opt = usvg::Options::default();
    // Get file's absolute directory.
    opt.resources_dir = std::fs::canonicalize(&args[1])
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    let mut fontdb = fontdb::Database::new();
    fontdb.load_system_fonts();

    let svg_data = std::fs::read(&args[1]).unwrap();
    let mut tree = usvg::Tree::from_data(&svg_data, &opt).unwrap();
    tree.convert_text(&fontdb, opt.keep_named_groups);

    let pixmap_size = tree.size.to_screen_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
    resvg::render(
        &tree,
        usvg::FitTo::Original,
        tiny_skia::Transform::default(),
        pixmap.as_mut(),
    )
    .unwrap();
    pixmap.save_png(&args[2]).unwrap();
}
