use std::rc::Rc;

use usvg::{fontdb, NodeExt, TreeParsing, TreeTextToPath};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if !(args.len() == 3 || args.len() == 5) {
        println!(
            "Usage:\n\
             \tdraw_bboxes <in-svg> <out-png>\n\
             \tdraw_bboxes <in-svg> <out-png> -z ZOOM"
        );
        return;
    }

    let zoom = if args.len() == 5 {
        args[4].parse::<f32>().expect("not a float")
    } else {
        1.0
    };

    let mut opt = usvg::Options::default();
    // Get file's absolute directory.
    opt.resources_dir = std::fs::canonicalize(&args[1])
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));
    // let fit_to = resvg::FitTo::Zoom(zoom);

    let mut fontdb = fontdb::Database::new();
    fontdb.load_system_fonts();

    let svg_data = std::fs::read(&args[1]).unwrap();
    let mut tree = usvg::Tree::from_data(&svg_data, &opt).unwrap();
    tree.convert_text(&fontdb);

    let mut bboxes = Vec::new();
    let mut text_bboxes = Vec::new();
    for node in tree.root.descendants() {
        if let Some(bbox) = node.calculate_bbox() {
            bboxes.push(bbox);
        }

        // Text bboxes are different from path bboxes.
        if let usvg::NodeKind::Text(ref text) = *node.borrow() {
            if let Some(ref bbox) = text.bounding_box {
                text_bboxes.push(bbox.to_rect());
            }
        }
    }

    let stroke = Some(usvg::Stroke {
        paint: usvg::Paint::Color(usvg::Color::new_rgb(255, 0, 0)),
        opacity: usvg::Opacity::new_clamped(0.5),
        ..usvg::Stroke::default()
    });

    let stroke2 = Some(usvg::Stroke {
        paint: usvg::Paint::Color(usvg::Color::new_rgb(0, 0, 200)),
        opacity: usvg::Opacity::new_clamped(0.5),
        ..usvg::Stroke::default()
    });

    for bbox in bboxes {
        let mut path = usvg::Path::new(Rc::new(tiny_skia::PathBuilder::from_rect(bbox)));
        path.stroke = stroke.clone();
        tree.root.append_kind(usvg::NodeKind::Path(path));
    }

    for bbox in text_bboxes {
        let mut path = usvg::Path::new(Rc::new(tiny_skia::PathBuilder::from_rect(bbox)));
        path.stroke = stroke2.clone();
        tree.root.append_kind(usvg::NodeKind::Path(path));
    }

    let rtree = resvg::Tree::from_usvg(&tree);

    let pixmap_size = rtree.size.to_int_size().scale_by(zoom).unwrap();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
    let render_ts = tiny_skia::Transform::from_scale(zoom, zoom);
    rtree.render(render_ts, &mut pixmap.as_mut());
    pixmap.save_png(&args[2]).unwrap();
}
