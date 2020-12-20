use std::rc::Rc;

use usvg::{NodeExt, SystemFontDB};

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
    opt.fontdb.set_generic_families();
    let fit_to = usvg::FitTo::Zoom(zoom);

    let rtree = usvg::Tree::from_file(&args[1], &opt).unwrap();

    let mut bboxes = Vec::new();
    for node in rtree.root().descendants() {
        if !rtree.is_in_defs(&node) {
            if let Some(bbox) = node.calculate_bbox() {
                bboxes.push(bbox);
            }
        }
    }

    let stroke = Some(usvg::Stroke {
        paint: usvg::Paint::Color(usvg::Color::new(255, 0, 0)),
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

    let pixmap_size = fit_to.fit_to(rtree.svg_node().size.to_screen_size()).unwrap();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
    resvg::render(&rtree, fit_to, pixmap.as_mut()).unwrap();
    pixmap.save_png(&args[2]).unwrap();
}
