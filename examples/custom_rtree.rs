use std::rc::Rc;

use usvg::NodeExt;

fn main() {
    let size = usvg::Size::new(200.0, 200.0).unwrap();
    let mut rtree = usvg::Tree::create(usvg::Svg {
        size,
        view_box: usvg::ViewBox {
            rect: size.to_rect(0.0, 0.0),
            aspect: usvg::AspectRatio::default(),
        },
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
                    color: usvg::Color::new_rgb(0, 255, 0),
                    opacity: usvg::Opacity::new(1.0),
                },
                usvg::Stop {
                    offset: usvg::StopOffset::new(1.0),
                    color: usvg::Color::new_rgb(0, 255, 0),
                    opacity: usvg::Opacity::new(0.0),
                },
            ],
        },
    }));

    let fill = Some(usvg::Fill {
        paint: usvg::Paint::Link("lg1".into()),
        ..usvg::Fill::default()
    });

    rtree.root().append_kind(usvg::NodeKind::Path(usvg::Path {
        fill,
        data: Rc::new(usvg::PathData::from_rect(usvg::Rect::new(20.0, 20.0, 160.0, 160.0).unwrap())),
        .. usvg::Path::default()
    }));

    #[cfg(feature = "dump-svg")]
    {
        println!("{}", rtree.to_string(&usvg::XmlOptions::default()));
    }

    let pixmap_size = rtree.svg_node().size.to_screen_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
    resvg::render(&rtree, usvg::FitTo::Original, pixmap.as_mut()).unwrap();
    pixmap.save_png("out.png").unwrap();
}
