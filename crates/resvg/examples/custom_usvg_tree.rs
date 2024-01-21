use std::rc::Rc;

use usvg::{fontdb, TreePostProc};

fn main() {
    let size = usvg::Size::from_wh(200.0, 200.0).unwrap();
    let mut tree = usvg::Tree {
        size,
        view_box: usvg::ViewBox {
            rect: size.to_non_zero_rect(0.0, 0.0),
            aspect: usvg::AspectRatio::default(),
        },
        root: usvg::Group::default(),
    };

    let gradient = usvg::LinearGradient {
        x1: 0.0,
        y1: 0.0,
        x2: 1.0,
        y2: 0.0,
        base: usvg::BaseGradient {
            id: "lg1".into(),
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

    let mut path = usvg::Path::new(Rc::new(tiny_skia::PathBuilder::from_rect(
        tiny_skia::Rect::from_xywh(20.0, 20.0, 160.0, 160.0).unwrap(),
    )));
    path.fill = fill;
    tree.root.children.push(usvg::Node::Path(Box::new(path)));
    tree.postprocess(
        usvg::PostProcessingSteps::default(),
        &fontdb::Database::new(),
    );

    let pixmap_size = tree.size.to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.save_png("out.png").unwrap();
}
