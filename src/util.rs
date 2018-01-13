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

pub fn split_into_vec(string: &str, delim: &str) -> Vec<String> {
    string
        .split(delim)
        .map(|s| s.to_string())
        .skip_while(|s| s.is_empty())
        .collect()
}

pub fn concat_by_delimeter(tags: &[String], delim: char) -> String {
    tags.iter().map(|tag| format!("{}{}", tag, delim)).collect()
}
