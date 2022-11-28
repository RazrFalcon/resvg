use std::rc::Rc;

use usvg::NodeExt;

fn main() {
    let size = usvg::Size::new(200.0, 200.0).unwrap();
    let tree = usvg::Tree {
        size,
        view_box: usvg::ViewBox {
            rect: size.to_rect(0.0, 0.0),
            aspect: usvg::AspectRatio::default(),
        },
        root: usvg::Node::new(usvg::NodeKind::Group(usvg::Group::default())),
    };

    let gradient = usvg::LinearGradient {
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
                    offset: usvg::StopOffset::ZERO,
                    color: usvg::Color::new_rgb(0, 255, 0),
                    opacity: usvg::Opacity::ONE,
                },
                usvg::Stop {
                    offset: usvg::StopOffset::ONE,
                    color: usvg::Color::new_rgb(0, 255, 0),
                    opacity: usvg::Opacity::ZERO,
                },
            ],
        },
    };

    let fill = Some(usvg::Fill {
        paint: usvg::Paint::LinearGradient(Rc::new(gradient)),
        ..usvg::Fill::default()
    });

    tree.root.append_kind(usvg::NodeKind::Path(usvg::Path {
        fill,
        data: Rc::new(usvg::PathData::from_rect(
            usvg::Rect::new(20.0, 20.0, 160.0, 160.0).unwrap(),
        )),
        ..usvg::Path::default()
    }));

    #[cfg(feature = "dump-svg")]
    {
        println!("{}", rtree.to_string(&usvg::XmlOptions::default()));
    }

    let pixmap_size = tree.size.to_screen_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
    resvg::render(
        &tree,
        usvg::FitTo::Original,
        tiny_skia::Transform::default(),
        pixmap.as_mut(),
    )
    .unwrap();
    pixmap.save_png("out.png").unwrap();
}
