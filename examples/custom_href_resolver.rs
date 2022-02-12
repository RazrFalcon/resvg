fn main() {
    let mut opt = usvg::Options::default();

    let ferris_image = std::sync::Arc::new(std::fs::read("./examples/ferris.png").unwrap());

    // We know that our SVG won't have DataUrl hrefs, just return None for such case.
    let resolve_data = Box::new(|_: &str, _: std::sync::Arc<Vec<u8>>, _: &usvg::OptionsRef| None);

    // Here we handle xlink:href attribute as string,
    // let's use already loaded Ferris image to match that string.
    let resolve_string = Box::new(move |href: &str, _: &usvg::OptionsRef| match href {
        "ferris_image" => Some(usvg::ImageKind::PNG(ferris_image.clone())),
        _ => None,
    });

    // Assign new ImageHrefResolver option using our closures.
    opt.image_href_resolver = usvg::ImageHrefResolver {
        resolve_data,
        resolve_string,
    };

    let svg_data = std::fs::read("./examples/custom_href_resolver.svg").unwrap();
    let rtree = usvg::Tree::from_data(&svg_data, &opt.to_ref()).unwrap();

    let pixmap_size = rtree.svg_node().size.to_screen_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();

    resvg::render(&rtree, usvg::FitTo::Original, tiny_skia::Transform::default(), pixmap.as_mut()).unwrap();

    pixmap.save_png("custom_href_resolver.png").unwrap();
}
