#[derive(Fail, Debug)]
pub enum MALError {
    #[fail(display = "minidom error")]
    Minidom(#[cause] ::minidom::error::Error),

    #[fail(display = "request error")]
    Request(#[cause] RequestError),

    #[fail(display = "list error")]
    List(#[cause] ListError),
}

#[derive(Fail, Debug)]
pub enum RequestError {
    #[fail(display = "error sending request to MAL")]
    HttpError(#[cause] ::reqwest::Error),

    #[fail(display = "failed to read response text")]
    ReadResponse(#[cause] ::reqwest::Error),

    #[fail(display = "received bad response code from MAL: {}", _0)]
    BadResponseCode(::reqwest::StatusCode),
}

#[derive(Fail, Debug)]
pub enum ListError {
    #[fail(display = "io error")]
    Io(#[cause] ::std::io::Error),

    #[fail(display = "minidom error")]
    Minidom(#[cause] ::minidom::error::Error),

    #[fail(display = "error converting data to UTF8")]
    Utf8(#[cause] ::std::string::FromUtf8Error),

    #[fail(display = "no user info found")]
    NoUserInfoFound,

    #[fail(display = "\"{}\" does not map to a known series status", _0)]
    UnknownStatus(String),

    #[fail(display = "\"{}\" does not map to a known series type", _0)]
    UnknownSeriesType(String),

    #[fail(display = "no XML node named \"{}\"", _0)]
    MissingXMLNode(String),

    #[fail(display = "failed to parse XML node \"{}\" into appropriate type", _0)]
    XMLConversionFailed(String),
}
