use std::rc::Rc;

use usvg::{fontdb, TreeParsing, TreePostProc};

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

    let mut fontdb = fontdb::Database::new();
    fontdb.load_system_fonts();

    let svg_data = std::fs::read(&args[1]).unwrap();
    let mut tree = usvg::Tree::from_data(&svg_data, &opt).unwrap();
    tree.postprocess(usvg::PostProcessingSteps::default(), &fontdb);

    let mut bboxes = Vec::new();
    let mut stroke_bboxes = Vec::new();
    collect_bboxes(&tree.root, &mut bboxes, &mut stroke_bboxes);

    let stroke1 = Some(usvg::Stroke {
        paint: usvg::Paint::Color(usvg::Color::new_rgb(255, 0, 0)),
        opacity: usvg::Opacity::new_clamped(0.5),
        ..usvg::Stroke::default()
    });

    let stroke2 = Some(usvg::Stroke {
        paint: usvg::Paint::Color(usvg::Color::new_rgb(0, 200, 0)),
        opacity: usvg::Opacity::new_clamped(0.5),
        ..usvg::Stroke::default()
    });

    for bbox in bboxes {
        let mut path = usvg::Path::new(Rc::new(tiny_skia::PathBuilder::from_rect(bbox)));
        path.stroke = stroke1.clone();
        tree.root.children.push(usvg::Node::Path(Box::new(path)));
    }

    for bbox in stroke_bboxes {
        let mut path = usvg::Path::new(Rc::new(tiny_skia::PathBuilder::from_rect(bbox)));
        path.stroke = stroke2.clone();
        tree.root.children.push(usvg::Node::Path(Box::new(path)));
    }

    // Calculate bboxes of newly added path.
    tree.calculate_bounding_boxes();

    let pixmap_size = tree.size.to_int_size().scale_by(zoom).unwrap();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
    let render_ts = tiny_skia::Transform::from_scale(zoom, zoom);
    resvg::render(&tree, render_ts, &mut pixmap.as_mut());
    pixmap.save_png(&args[2]).unwrap();
}

fn collect_bboxes(
    parent: &usvg::Group,
    bboxes: &mut Vec<usvg::Rect>,
    stroke_bboxes: &mut Vec<usvg::Rect>,
) {
    for node in &parent.children {
        if let usvg::Node::Group(ref group) = node {
            collect_bboxes(group, bboxes, stroke_bboxes);
        }

        if let Some(bbox) = node.abs_bounding_box() {
            bboxes.push(bbox);

            if let Some(stroke_bbox) = node.abs_stroke_bounding_box() {
                if bbox != stroke_bbox.to_rect() {
                    stroke_bboxes.push(stroke_bbox.to_rect());
                }
            }
        }
    }
}
