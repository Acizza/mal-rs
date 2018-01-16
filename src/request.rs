use MAL;
use reqwest::{self, RequestBuilder, Response, StatusCode, Url};
use reqwest::header::{ContentType, Headers};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ListType {
    Anime,
    Manga,
}

#[derive(Fail, Debug)]
pub enum RequestError {
    #[fail(display = "received bad response code from MAL: {}", _0)] BadResponseCode(StatusCode),
    #[fail(display = "error sending request to MAL: {}", _0)] HttpError(#[cause] reqwest::Error),
}

pub type ID = u32;
pub type Username<'a> = &'a str;
pub type Name<'a> = &'a str;
pub type Body<'a> = &'a str;

#[derive(Debug)]
pub enum Request<'a> {
    Search(Name<'a>, ListType),
    List(Username<'a>, ListType),
    Add(ID, ListType, Body<'a>),
    Update(ID, ListType, Body<'a>),
    Delete(ID, ListType),
    VerifyCredentials,
}

impl<'a> Request<'a> {
    pub const BASE_URL: &'static str = "https://myanimelist.net";

    pub fn send(self, mal: &MAL) -> Result<Response, RequestError> {
        lazy_static! {
            static ref BASE_URL: Url = Url::parse(Request::BASE_URL).unwrap();
        }

        let mut url = BASE_URL.clone();
        use self::Request::*;

        match self {
            Search(name, list_type) => {
                match list_type {
                    ListType::Anime => url.set_path("/api/anime/search.xml"),
                    ListType::Manga => url.set_path("/api/manga/search.xml"),
                }

                url.query_pairs_mut().append_pair("q", name);
                mal.client.get(url).with_auth(mal).send_req()
            }
            List(uname, list_type) => {
                url.set_path("/malappinfo.php");

                let query = match list_type {
                    ListType::Anime => "anime",
                    ListType::Manga => "manga",
                };

                url.query_pairs_mut()
                    .append_pair("u", uname)
                    .append_pair("status", "all")
                    .append_pair("type", query);

                mal.client.get(url).send_req()
            }
            Add(id, list_type, body) => {
                match list_type {
                    ListType::Anime => url.set_path(&format!("/api/animelist/add/{}.xml", id)),
                    ListType::Manga => url.set_path(&format!("/api/mangalist/add/{}.xml", id)),
                }

                mal.client
                    .post(url)
                    .with_body(body)
                    .with_auth(mal)
                    .send_req()
            }
            Update(id, list_type, body) => {
                match list_type {
                    ListType::Anime => url.set_path(&format!("/api/animelist/update/{}.xml", id)),
                    ListType::Manga => url.set_path(&format!("/api/mangalist/update/{}.xml", id)),
                }

                mal.client
                    .post(url)
                    .with_body(body)
                    .with_auth(mal)
                    .send_req()
            }
            Delete(id, list_type) => {
                match list_type {
                    ListType::Anime => url.set_path(&format!("/api/animelist/delete/{}.xml", id)),
                    ListType::Manga => url.set_path(&format!("/api/mangalist/delete/{}.xml", id)),
                }

                mal.client.delete(url).with_auth(mal).send_req()
            }
            VerifyCredentials => {
                url.set_path("/api/account/verify_credentials.xml");
                mal.client.get(url).with_auth(mal).send_req()
            }
        }
    }
}

trait RequestExt {
    fn with_auth(&mut self, mal: &MAL) -> &mut RequestBuilder;
    fn with_body(&mut self, body: &str) -> &mut RequestBuilder;

    fn send_req(&mut self) -> Result<Response, RequestError>;
}

impl RequestExt for RequestBuilder {
    fn with_auth(&mut self, mal: &MAL) -> &mut RequestBuilder {
        self.basic_auth(mal.username.clone(), Some(mal.password.clone()))
    }

    fn with_body(&mut self, body: &str) -> &mut RequestBuilder {
        let mut headers = Headers::new();
        headers.set(ContentType::form_url_encoded());

        self.body(format!("data={}", body)).headers(headers)
    }

    fn send_req(&mut self) -> Result<Response, RequestError> {
        let resp = self.send().map_err(RequestError::HttpError)?;

        match resp.status() {
            StatusCode::Ok | StatusCode::Created => Ok(resp),
            status => Err(RequestError::BadResponseCode(status)),
        }
    }
}
