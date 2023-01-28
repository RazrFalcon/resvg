// Attributes parsing example.
//
// While rosvgtree doesn't parse attribute values by default,
// it does provide ways to simplify it.

const SVG: &str = "
<svg width='200' height='300mm' xmlns='http://www.w3.org/2000/svg'/>
";

use rosvgtree::{svgtypes, AttributeId};

struct CustomType;
impl<'a, 'input: 'a> rosvgtree::FromValue<'a, 'input> for CustomType {
    fn parse(_: rosvgtree::Node, aid: AttributeId, value: &str) -> Option<Self> {
        println!("parsing {}='{}'", aid, value);
        Some(CustomType)
    }
}

fn main() {
    let svg = rosvgtree::Document::parse_str(SVG).unwrap();

    let svg_elem = svg.root_element();

    // Get attribute value as a string via type inference.
    assert_eq!(svg_elem.attribute(AttributeId::Width), Some("200"));

    // Get attribute value as a string using an explicit type.
    assert_eq!(svg_elem.attribute::<&str>(AttributeId::Width), Some("200"));

    // Parse attribute value as a number.
    assert_eq!(svg_elem.attribute::<f64>(AttributeId::Width), Some(200.0));

    // Parse attribute value as a length.
    // Most `svgtypes` types are supported.
    assert_eq!(
        svg_elem.attribute::<svgtypes::Length>(AttributeId::Height),
        Some(svgtypes::Length::new(300.0, svgtypes::LengthUnit::Mm)),
    );

    // Parse attribute value into a custom type.
    svg_elem.attribute::<CustomType>(AttributeId::Width);
}
