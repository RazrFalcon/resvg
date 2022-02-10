use std::sync::Arc;

use usvg::{ImageHrefResolver, ImageKind};

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
    opt.fontdb.load_system_fonts();

    // We know that our SVG won't have DataUrl hrefs.
    let resolve_data = Box::new(|_: &str, _: Arc<Vec<u8>>| None);

    // Let's treat href as URL and download PNG image via network.
    let resolve_string = Box::new(|href: &str| {
        reqwest::blocking::get(href)
            .and_then(|data| data.bytes())
            .map(|bytes| bytes.to_vec())
            .ok()
            .map(|data: Vec<u8>| ImageKind::PNG(Arc::new(data)))
    });

    // Assign new ImageHrefResolver option using our closures.
    // Now `href` strings in `<image>` elements will be treated as URLs to PNG images.
    opt.image_href_resolver = Some(ImageHrefResolver {
        resolve_data,
        resolve_string,
    });

    let svg_data = std::fs::read(&args[1]).unwrap();
    let rtree = usvg::Tree::from_data(&svg_data, &opt.to_ref()).unwrap();

    let pixmap_size = rtree.svg_node().size.to_screen_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();

    resvg::render(
        &rtree,
        usvg::FitTo::Original,
        tiny_skia::Transform::default(),
        pixmap.as_mut(),
    )
    .unwrap();

    pixmap.save_png(&args[2]).unwrap();
}
