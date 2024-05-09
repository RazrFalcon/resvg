use usvgr::PreloadedImageData;

fn main() {
    let mut opt = usvgr::Options::default();

    let ferris_image = std::fs::read("./examples/ferris.png").unwrap();
    let ferris_image = image::load_from_memory(ferris_image.as_slice()).unwrap();

    let preloaded_data = std::collections::HashMap::from([(
        "ferris_image".to_owned(),
        std::sync::Arc::new(PreloadedImageData::new(
            "png".to_string(),
            ferris_image.width(),
            ferris_image.height(),
            &ferris_image.to_rgba8().into_raw(),
        )),
    )]);

    opt.image_data = Some(&preloaded_data);

    let fontdb = usvgr::fontdb::Database::new();

    let svg_data = std::fs::read("./examples/custom_href_resolver.svg").unwrap();
    let tree = usvgr::Tree::from_data(&svg_data, &opt, &fontdb).unwrap();

    let pixmap_size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();

    let mut cache = svgr::SvgrCache::new(10);
    let ctx = svgr::Context::new_from_pixmap(&pixmap);
    svgr::render(
        &tree,
        tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
        &mut cache,
        &ctx,
    );

    pixmap.save_png("custom_href_resolver.png").unwrap();
}
