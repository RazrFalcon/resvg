use std::sync::Arc;

use usvg::{ImageHrefResolver, ImageKind, OptionsRef};

fn main() {
    let mut opt = usvg::Options::default();

    let ferris_image_path = "./examples/ferris.png";
    let ferris_image = Arc::new(std::fs::read(ferris_image_path).unwrap());

    // We know that our SVG won't have DataUrl hrefs, just return None for such case.
    let resolve_data = Box::new(|_: &str, _: Arc<Vec<u8>>, _: &OptionsRef| None);

    // Here we handle xlink:href attribute as string, let's use already loaded Ferris image to match that string.
    let resolve_string = Box::new(move |href: &str, _: &OptionsRef| match href {
        "ferris_image" => Some(ImageKind::PNG(ferris_image.clone())),
        _ => None
    });

    // Assign new ImageHrefResolver option using our closures.
    opt.image_href_resolver = ImageHrefResolver {
        resolve_data,
        resolve_string,
    };

    let svg_with_ferris_path = "./examples/custom_href_resolver.svg";
    let svg_data = std::fs::read(svg_with_ferris_path).unwrap();
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

    pixmap.save_png("custom_href_resolver.png").unwrap();
}
