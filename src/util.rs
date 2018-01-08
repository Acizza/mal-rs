use minidom::Element;

#[derive(Fail, Debug)]
#[fail(display = "no XML node named '{}'", _0)]
pub struct MissingXMLNode(pub String);

pub fn get_xml_child_text(elem: &Element, name: &str) -> Result<String, MissingXMLNode> {
    elem.children()
        .find(|c| c.name() == name)
        .map(|c| c.text())
        .ok_or_else(|| MissingXMLNode(name.into()))
}
