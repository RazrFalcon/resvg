use usvg::FuzzyEq;

#[test]
fn clippath_with_invalid_child() {
    let svg = "
    <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
        <clipPath id='clip1'>
            <rect/>
        </clipPath>
        <rect clip-path='url(#clip1)' width='10' height='10'/>
    </svg>
    ";

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    // clipPath is invalid and should be removed together with rect.
    assert_eq!(tree.root.has_children(), false);
}

#[test]
fn simplify_paths() {
    let svg = "
    <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
        <path d='M 10 20 L 10 30 Z Z Z'/>
    </svg>
    ";

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    let path = tree.root.first_child().unwrap();
    match *path.borrow() {
        usvg::NodeKind::Path(ref path) => {
            // Make use we have MLZ and not MLZZZ
            assert_eq!(path.data.commands().len(), 3);
        }
        _ => unreachable!(),
    };
}

#[test]
fn size_detection_1() {
    let svg = "<svg viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'/>";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert!(tree.size.fuzzy_eq(&usvg::Size::new(10.0, 20.0).unwrap()));
}

#[test]
fn size_detection_2() {
    let svg =
        "<svg width='30' height='40' viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'/>";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert!(tree.size.fuzzy_eq(&usvg::Size::new(30.0, 40.0).unwrap()));
}

#[test]
fn size_detection_3() {
    let svg =
        "<svg width='50%' height='100%' viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'/>";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert!(tree.size.fuzzy_eq(&usvg::Size::new(5.0, 20.0).unwrap()));
}

#[test]
fn size_detection_4() {
    let svg = "
    <svg xmlns='http://www.w3.org/2000/svg'>
        <circle cx='18' cy='18' r='18'/>
    </svg>
    ";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert!(tree.size.fuzzy_eq(&usvg::Size::new(36.0, 36.0).unwrap()));
    assert!(tree
        .view_box
        .rect
        .fuzzy_eq(&usvg::Rect::new(0.0, 0.0, 36.0, 36.0).unwrap()));
}

#[test]
fn size_detection_5() {
    let svg = "<svg xmlns='http://www.w3.org/2000/svg'/>";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert!(tree.size.fuzzy_eq(&usvg::Size::new(100.0, 100.0).unwrap()));
}

#[test]
fn invalid_size_1() {
    let svg = "<svg width='0' height='0' viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'/>";
    let result = usvg::Tree::from_str(&svg, &usvg::Options::default());
    assert!(result.is_err());
}
