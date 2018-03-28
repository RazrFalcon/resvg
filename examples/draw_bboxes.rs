extern crate resvg;

use std::env;
use std::path::Path;

use resvg::{
    utils,
    Options,
};
use resvg::tree::prelude::*;

// TODO: write doc

fn main() {
    let args: Vec<String> = env::args().collect();
    if !(args.len() == 3 || args.len() == 5) {
        println!("Usage:\n\
            \tdraw_bboxes <in-svg> <out-png>\n\
            \tdraw_bboxes <in-svg> <out-png> -z ZOOM");
        return;
    }

    let _resvg = resvg::init();
    let backend = resvg::default_backend();

    let zoom = if args.len() == 5 {
        args[4].parse::<f64>().expect("not a float")
    } else {
        1.0
    };

    let mut opt = Options::default();
    opt.usvg.path = Some(args[1].clone().into());
    opt.usvg.keep_named_groups = true;
    opt.fit_to = resvg::FitTo::Zoom(zoom as f32);

    let mut rtree = resvg::parse_rtree_from_file(&args[1], &opt).unwrap();

    let mut bboxes = Vec::new();
    for node in rtree.root().descendants() {
        if !rtree.is_in_defs(node) {
            if let Some(bbox) = backend.calc_node_bbox(node, &opt) {
                bboxes.push(bbox);
            }
        }
    }

    let stroke = Some(tree::Stroke {
        paint: tree::Paint::Color(tree::Color::new(255, 0, 0)),
        opacity: 0.5.into(),
        .. tree::Stroke::default()
    });

    for bbox in bboxes {
        let mut root = rtree.root_mut();
        root.append(Box::new(tree::NodeKind::Path(tree::Path {
            id: String::new(),
            transform: tree::Transform::default(),
            fill: None,
            stroke: stroke.clone(),
            segments: utils::rect_to_path(bbox),
        })));
    }

    let img = backend.render_to_image(&rtree, &opt).unwrap();
    img.save(Path::new(&args[2]));
}
