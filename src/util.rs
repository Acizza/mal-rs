use chrono::NaiveDate;
use minidom::Element;
use std::str::FromStr;

#[derive(Fail, Debug)]
pub enum ParseXMLError {
    #[fail(display = "no XML node named \"{}\"", _0)]
    MissingXMLNode(String),
    
    #[fail(display = "failed to parse XML node \"{}\" into appropriate type", _0)]
    ConversionFailed(String),
}

pub fn parse_xml_child<T: FromStr>(elem: &Element, name: &str) -> Result<T, ParseXMLError> {
    let text = elem.children()
        .find(|c| c.name() == name)
        .ok_or_else(|| ParseXMLError::MissingXMLNode(name.into()))?
        .text();

    text.parse::<T>()
        .map_err(|_| ParseXMLError::ConversionFailed(name.into()))
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
