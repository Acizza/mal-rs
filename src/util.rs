use chrono::NaiveDate;
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

pub fn parse_str_date(date: &str) -> Option<NaiveDate> {
    if date != "0000-00-00" {
        NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()
    } else {
        None
    }
}

pub fn date_to_str(date: Option<NaiveDate>) -> String {
    match date {
        Some(date) => date.format("%m%d%Y").to_string(),
        None => {
            // MAL uses an all-zero date to represent a non-set one
            "00000000".into()
        }
    }
}
