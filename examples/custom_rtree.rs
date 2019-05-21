use std::path::Path;

use resvg::prelude::*;

// TODO: write doc

fn main() {
    let _resvg = resvg::init();
    let backend = resvg::default_backend();

    let opt = Options::default();

    let size = Size::new(200.0, 200.0).unwrap();
    let view_box = usvg::ViewBox {
        rect: size.to_rect(0.0, 0.0),
        aspect: usvg::AspectRatio::default(),
    };

    let mut rtree = usvg::Tree::create(usvg::Svg {
        size,
        view_box,
    });

    rtree.append_to_defs(usvg::NodeKind::LinearGradient(usvg::LinearGradient {
        id: "lg1".into(),
        x1: 0.0,
        y1: 0.0,
        x2: 1.0,
        y2: 0.0,
        base: usvg::BaseGradient {
            units: usvg::Units::ObjectBoundingBox,
            transform: usvg::Transform::default(),
            spread_method: usvg::SpreadMethod::Pad,
            stops: vec![
                usvg::Stop {
                    offset: usvg::StopOffset::new(0.0),
                    color: usvg::Color::new(0, 255, 0),
                    opacity: usvg::Opacity::new(1.0),
                },
                usvg::Stop {
                    offset: usvg::StopOffset::new(1.0),
                    color: usvg::Color::new(0, 255, 0),
                    opacity: usvg::Opacity::new(0.0),
                },
            ],
        },
    }));

    let fill = Some(usvg::Fill {
        paint: usvg::Paint::Link("lg1".into()),
        .. usvg::Fill::default()
    });

    rtree.root().append_kind(usvg::NodeKind::Path(usvg::Path {
        id: String::new(),
        transform: usvg::Transform::default(),
        visibility: usvg::Visibility::Visible,
        fill,
        stroke: None,
        rendering_mode: usvg::ShapeRendering::default(),
        segments: utils::rect_to_path(Rect::new(20.0, 20.0, 160.0, 160.0).unwrap()),
    }));

    println!("{}", rtree.to_svgdom());

    let img = backend.render_to_image(&rtree, &opt).unwrap();
    img.save(Path::new("out.png"));
}
