use resvg::prelude::*;

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
        args[4].parse::<f64>().expect("not a float")
    } else {
        1.0
    };

    let mut opt = Options::default();
    opt.usvg.path = Some(args[1].clone().into());
    opt.usvg.keep_named_groups = true;
    opt.fit_to = resvg::FitTo::Zoom(zoom as f32);

    let rtree = usvg::Tree::from_file(&args[1], &opt.usvg).unwrap();

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
            id: String::new(),
            transform: usvg::Transform::default(),
            visibility: usvg::Visibility::Visible,
            fill: None,
            stroke: stroke.clone(),
            rendering_mode: usvg::ShapeRendering::default(),
            segments: utils::rect_to_path(bbox),
        }));
    }

    let img = resvg::default_backend().render_to_image(&rtree, &opt).unwrap();
    img.save(std::path::Path::new(&args[2]));
}
