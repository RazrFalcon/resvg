use std::rc::Rc;

use usvg::NodeExt;

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
    opt.resources_dir = std::fs::canonicalize(&args[1]).ok().and_then(|p| p.parent().map(|p| p.to_path_buf()));
    opt.keep_named_groups = true;
    opt.fontdb.load_system_fonts();
    let fit_to = usvg::FitTo::Zoom(zoom);

    let svg_data = std::fs::read(&args[1]).unwrap();
    let rtree = usvg::Tree::from_data(&svg_data, &opt.to_ref()).unwrap();

    let mut bboxes = Vec::new();
    let mut text_bboxes = Vec::new();
    for node in rtree.root().descendants() {
        if !rtree.is_in_defs(&node) {
            if let Some(bbox) = node.calculate_bbox().and_then(|r| r.to_rect()) {
                bboxes.push(bbox);
            }

            // Text bboxes are different from path bboxes.
            if let usvg::NodeKind::Path(ref path) = *node.borrow() {
                if let Some(ref bbox) = path.text_bbox {
                    text_bboxes.push(*bbox);
                }
            }
        }
    }

    let stroke = Some(usvg::Stroke {
        paint: usvg::Paint::Color(usvg::Color::new_rgb(255, 0, 0)),
        opacity: 0.5.into(),
        .. usvg::Stroke::default()
    });

    let stroke2 = Some(usvg::Stroke {
        paint: usvg::Paint::Color(usvg::Color::new_rgb(0, 0, 200)),
        opacity: 0.5.into(),
        .. usvg::Stroke::default()
    });

    for bbox in bboxes {
        rtree.root().append_kind(usvg::NodeKind::Path(usvg::Path {
            stroke: stroke.clone(),
            data: Rc::new(usvg::PathData::from_rect(bbox)),
            .. usvg::Path::default()
        }));
    }

    for bbox in text_bboxes {
        rtree.root().append_kind(usvg::NodeKind::Path(usvg::Path {
            stroke: stroke2.clone(),
            data: Rc::new(usvg::PathData::from_rect(bbox)),
            .. usvg::Path::default()
        }));
    }

    let pixmap_size = fit_to.fit_to(rtree.svg_node().size.to_screen_size()).unwrap();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
    resvg::render(&rtree, fit_to, pixmap.as_mut()).unwrap();
    pixmap.save_png(&args[2]).unwrap();
}
