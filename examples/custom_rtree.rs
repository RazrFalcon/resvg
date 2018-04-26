extern crate resvg;

use std::path::Path;

use resvg::{
    tree,
    utils,
    Options,
};
use resvg::tree::prelude::*;
use resvg::geom::*;

// TODO: write doc

fn main() {
    let _resvg = resvg::init();
    let backend = resvg::default_backend();

    let opt = Options::default();

    let size = Size::new(200.0, 200.0);
    let view_box = tree::ViewBox {
        rect: Rect::new(Point::new(0.0, 0.0), size),
        aspect: tree::AspectRatio::default(),
    };

    let mut rtree = tree::Tree::create(tree::Svg {
        size,
        view_box,
    });

    let mut grad = rtree.append_to_defs(tree::NodeKind::LinearGradient(tree::LinearGradient {
        id: "lg1".into(),
        x1: 0.0,
        y1: 0.0,
        x2: 1.0,
        y2: 0.0,
        d: tree::BaseGradient {
            units: tree::Units::ObjectBoundingBox,
            transform: tree::Transform::default(),
            spread_method: tree::SpreadMethod::Pad,
        },
    }));

    grad.append_kind(tree::NodeKind::Stop(tree::Stop {
        offset: tree::StopOffset::new(0.0),
        color: tree::Color::new(0, 255, 0),
        opacity: tree::Opacity::new(1.0),
    }));

    grad.append_kind(tree::NodeKind::Stop(tree::Stop {
        offset: tree::StopOffset::new(1.0),
        color: tree::Color::new(0, 255, 0),
        opacity: tree::Opacity::new(0.0),
    }));


    let fill = Some(tree::Fill {
        paint: tree::Paint::Link("lg1".into()),
        .. tree::Fill::default()
    });

    rtree.root().append_kind(tree::NodeKind::Path(tree::Path {
        id: String::new(),
        transform: tree::Transform::default(),
        fill,
        stroke: None,
        segments: utils::rect_to_path(Rect::from_xywh(20.0, 20.0, 160.0, 160.0)),
    }));

    println!("{}", rtree.to_svgdom());

    let img = backend.render_to_image(&rtree, &opt).unwrap();
    img.save(Path::new("out.png"));
}
