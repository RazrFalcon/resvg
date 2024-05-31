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
    assert_eq!(tree.root().has_children(), false);
}

#[test]
fn simplify_paths() {
    let svg = "
    <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1 1'>
        <path d='M 10 20 L 10 30 Z Z Z'/>
    </svg>
    ";

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    let path = &tree.root().children()[0];
    match path {
        usvg::Node::Path(ref path) => {
            // Make sure we have MLZ and not MLZZZ
            assert_eq!(path.data().verbs().len(), 3);
        }
        _ => unreachable!(),
    };
}

#[test]
fn size_detection_1() {
    let svg = "<svg viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'/>";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.size(), usvg::Size::from_wh(10.0, 20.0).unwrap());
}

#[test]
fn size_detection_2() {
    let svg =
        "<svg width='30' height='40' viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'/>";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.size(), usvg::Size::from_wh(30.0, 40.0).unwrap());
}

#[test]
fn size_detection_3() {
    let svg =
        "<svg width='50%' height='100%' viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'/>";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.size(), usvg::Size::from_wh(5.0, 20.0).unwrap());
}

#[test]
fn size_detection_4() {
    let svg = "
    <svg xmlns='http://www.w3.org/2000/svg'>
        <circle cx='18' cy='18' r='18'/>
    </svg>
    ";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.size(), usvg::Size::from_wh(36.0, 36.0).unwrap());
    assert_eq!(tree.size(), usvg::Size::from_wh(36.0, 36.0).unwrap());
}

#[test]
fn size_detection_5() {
    let svg = "<svg xmlns='http://www.w3.org/2000/svg'/>";
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.size(), usvg::Size::from_wh(100.0, 100.0).unwrap());
}

#[test]
fn invalid_size_1() {
    let svg = "<svg width='0' height='0' viewBox='0 0 10 20' xmlns='http://www.w3.org/2000/svg'/>";
    let result = usvg::Tree::from_str(&svg, &usvg::Options::default());
    assert!(result.is_err());
}

#[test]
fn tree_is_send_and_sync() {
    fn ensure_send_and_sync<T: Send + Sync>() {}
    ensure_send_and_sync::<usvg::Tree>();
}

#[test]
fn path_transform() {
    let svg = "
    <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>
        <path transform='translate(10)' d='M 0 0 L 10 10'/>
    </svg>
    ";

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.root().children().len(), 1);

    let group_node = &tree.root().children()[0];
    assert!(matches!(group_node, usvg::Node::Group(_)));
    assert_eq!(
        group_node.abs_transform(),
        usvg::Transform::from_translate(10.0, 0.0)
    );

    let group = match group_node {
        usvg::Node::Group(ref g) => g,
        _ => unreachable!(),
    };

    let path = &group.children()[0];
    assert!(matches!(path, usvg::Node::Path(_)));
    assert_eq!(
        path.abs_transform(),
        usvg::Transform::from_translate(10.0, 0.0)
    );
}

#[test]
fn path_transform_nested() {
    let svg = "
    <svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>
        <g transform='translate(20)'>
            <path transform='translate(10)' d='M 0 0 L 10 10'/>
        </g>
    </svg>
    ";

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();
    assert_eq!(tree.root().children().len(), 1);

    let group_node1 = &tree.root().children()[0];
    assert!(matches!(group_node1, usvg::Node::Group(_)));
    assert_eq!(
        group_node1.abs_transform(),
        usvg::Transform::from_translate(20.0, 0.0)
    );

    let group1 = match group_node1 {
        usvg::Node::Group(ref g) => g,
        _ => unreachable!(),
    };

    let group_node2 = &group1.children()[0];
    assert!(matches!(group_node2, usvg::Node::Group(_)));
    assert_eq!(
        group_node2.abs_transform(),
        usvg::Transform::from_translate(30.0, 0.0)
    );

    let group2 = match group_node2 {
        usvg::Node::Group(ref g) => g,
        _ => unreachable!(),
    };

    let path = &group2.children()[0];
    assert!(matches!(path, usvg::Node::Path(_)));
    assert_eq!(
        path.abs_transform(),
        usvg::Transform::from_translate(30.0, 0.0)
    );
}

#[test]
fn path_transform_in_symbol_no_clip() {
    let svg = "
    <svg viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'>
        <defs>
            <symbol id='symbol1' overflow='visible'>
                <rect id='rect1' x='0' y='0' width='10' height='10'/>
            </symbol>
        </defs>
        <use id='use1' xlink:href='#symbol1' x='20'/>
    </svg>
    ";

    // Will be parsed as:
    // <svg width="100" height="100" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    //     <g id="use1">
    //         <g transform="matrix(1 0 0 1 20 0)">
    //             <path fill="#000000" stroke="none" d="M 0 0 L 10 0 L 10 10 L 0 10 Z"/>
    //         </g>
    //     </g>
    // </svg>

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();

    let group_node1 = &tree.root().children()[0];
    assert!(matches!(group_node1, usvg::Node::Group(_)));
    assert_eq!(group_node1.id(), "use1");
    assert_eq!(group_node1.abs_transform(), usvg::Transform::default());

    let group1 = match group_node1 {
        usvg::Node::Group(ref g) => g,
        _ => unreachable!(),
    };

    let group_node2 = &group1.children()[0];
    assert!(matches!(group_node2, usvg::Node::Group(_)));
    assert_eq!(
        group_node2.abs_transform(),
        usvg::Transform::from_translate(20.0, 0.0)
    );

    let group2 = match group_node2 {
        usvg::Node::Group(ref g) => g,
        _ => unreachable!(),
    };

    let path = &group2.children()[0];
    assert!(matches!(path, usvg::Node::Path(_)));
    assert_eq!(
        path.abs_transform(),
        usvg::Transform::from_translate(20.0, 0.0)
    );
}

#[test]
fn path_transform_in_symbol_with_clip() {
    let svg = "
    <svg viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'>
        <defs>
            <symbol id='symbol1' overflow='hidden'>
                <rect id='rect1' x='0' y='0' width='10' height='10'/>
            </symbol>
        </defs>
        <use id='use1' xlink:href='#symbol1' x='20'/>
    </svg>
    ";

    // Will be parsed as:
    // <svg width="100" height="100" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    //     <defs>
    //         <clipPath id="clipPath1">
    //             <path fill="#000000" stroke="none" d="M 20 0 L 120 0 L 120 100 L 20 100 Z"/>
    //         </clipPath>
    //     </defs>
    //     <g id="use1" clip-path="url(#clipPath1)">
    //         <g>
    //             <g transform="matrix(1 0 0 1 20 0)">
    //                 <path fill="#000000" stroke="none" d="M 0 0 L 10 0 L 10 10 L 0 10 Z"/>
    //             </g>
    //         </g>
    //     </g>
    // </svg>

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();

    let group_node1 = &tree.root().children()[0];
    assert!(matches!(group_node1, usvg::Node::Group(_)));
    assert_eq!(group_node1.id(), "use1");
    assert_eq!(group_node1.abs_transform(), usvg::Transform::default());

    let group1 = match group_node1 {
        usvg::Node::Group(ref g) => g,
        _ => unreachable!(),
    };

    let group_node2 = &group1.children()[0];
    assert!(matches!(group_node2, usvg::Node::Group(_)));
    assert_eq!(group_node2.abs_transform(), usvg::Transform::default());

    let group2 = match group_node2 {
        usvg::Node::Group(ref g) => g,
        _ => unreachable!(),
    };

    let group_node3 = &group2.children()[0];
    assert!(matches!(group_node3, usvg::Node::Group(_)));
    assert_eq!(
        group_node3.abs_transform(),
        usvg::Transform::from_translate(20.0, 0.0)
    );

    let group3 = match group_node3 {
        usvg::Node::Group(ref g) => g,
        _ => unreachable!(),
    };

    let path = &group3.children()[0];
    assert!(matches!(path, usvg::Node::Path(_)));
    assert_eq!(
        path.abs_transform(),
        usvg::Transform::from_translate(20.0, 0.0)
    );
}

#[test]
fn path_transform_in_svg() {
    let svg = "
    <svg viewBox='0 0 100 100' xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink'>
        <g id='g1' transform='translate(100 150)'>
            <svg id='svg1' width='100' height='50'>
                <rect id='rect1' width='10' height='10'/>
            </svg>
        </g>
    </svg>
    ";

    // Will be parsed as:
    // <svg width="100" height="100" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
    //     <defs>
    //         <clipPath id="clipPath1">
    //             <path fill="#000000" stroke="none" d="M 0 0 L 100 0 L 100 50 L 0 50 Z"/>
    //         </clipPath>
    //     </defs>
    //     <g id="g1" transform="matrix(1 0 0 1 100 150)">
    //         <g id="svg1" clip-path="url(#clipPath1)">
    //             <path id="rect1" fill="#000000" stroke="none" d="M 0 0 L 10 0 L 10 10 L 0 10 Z"/>
    //         </g>
    //     </g>
    // </svg>

    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).unwrap();

    let group_node1 = &tree.root().children()[0];
    assert!(matches!(group_node1, usvg::Node::Group(_)));
    assert_eq!(group_node1.id(), "g1");
    assert_eq!(
        group_node1.abs_transform(),
        usvg::Transform::from_translate(100.0, 150.0)
    );

    let group1 = match group_node1 {
        usvg::Node::Group(ref g) => g,
        _ => unreachable!(),
    };

    let group_node2 = &group1.children()[0];
    assert!(matches!(group_node2, usvg::Node::Group(_)));
    assert_eq!(group_node2.id(), "svg1");
    assert_eq!(
        group_node2.abs_transform(),
        usvg::Transform::from_translate(100.0, 150.0)
    );

    let group2 = match group_node2 {
        usvg::Node::Group(ref g) => g,
        _ => unreachable!(),
    };

    let path = &group2.children()[0];
    assert!(matches!(path, usvg::Node::Path(_)));
    assert_eq!(
        path.abs_transform(),
        usvg::Transform::from_translate(100.0, 150.0)
    );
}
