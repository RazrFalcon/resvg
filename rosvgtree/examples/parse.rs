// Parse and write SVG back to XML to see how it was preprocessed.

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage:\n\tcargo run --example parse -- input.svg");
        std::process::exit(1);
    }

    let input = std::fs::read_to_string(&args[1]).unwrap();
    let svg = rosvgtree::Document::parse_str(&input).unwrap();

    let opt = xmlwriter::Options::default();
    let mut xml = xmlwriter::XmlWriter::new(opt);
    write_element(svg.root(), &mut xml);
    print!("{}", xml.end_document());
}

fn write_element(parent: rosvgtree::Node, xml: &mut xmlwriter::XmlWriter) {
    for node in parent.children() {
        let tag_name = match node.tag_name() {
            Some(v) => v,
            None => {
                let text = node.text();
                if !text.is_empty() {
                    xml.write_text(text);
                }

                continue;
            }
        };

        xml.start_element(tag_name.to_str());

        for attr in node.attributes() {
            xml.write_attribute(attr.name.to_str(), &attr.value);
        }

        if tag_name == rosvgtree::ElementId::Text {
            xml.set_preserve_whitespaces(true);
        }

        if node.has_children() {
            write_element(node, xml);
        }

        xml.end_element();

        if tag_name == rosvgtree::ElementId::Text {
            xml.set_preserve_whitespaces(false);
        }
    }
}
