use failure::Error;
use list::ListType;
use MAL;
use reqwest::{RequestBuilder, Response, StatusCode, Url};
use reqwest::header::{ContentType, Headers};

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

    pub fn send(self, mal: &MAL) -> Result<Response, Error> {
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
                Ok(mal.client.get(url).with_auth(mal).send()?)
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

                Ok(mal.client.get(url).send()?.verify_status()?)
            }
            Add(id, list_type, body) => {
                match list_type {
                    ListType::Anime => url.set_path(&format!("/api/animelist/add/{}.xml", id)),
                    ListType::Manga => url.set_path(&format!("/api/mangalist/add/{}.xml", id)),
                }

                Ok(mal.client
                    .post(url)
                    .with_body(body)
                    .with_auth(mal)
                    .send()?
                    .verify_status()?)
            }
            Update(id, list_type, body) => {
                match list_type {
                    ListType::Anime => url.set_path(&format!("/api/animelist/update/{}.xml", id)),
                    ListType::Manga => url.set_path(&format!("/api/mangalist/update/{}.xml", id)),
                }

                Ok(mal.client
                    .post(url)
                    .with_body(body)
                    .with_auth(mal)
                    .send()?
                    .verify_status()?)
            }
            Delete(id, list_type) => {
                match list_type {
                    ListType::Anime => url.set_path(&format!("/api/animelist/delete/{}.xml", id)),
                    ListType::Manga => url.set_path(&format!("/api/mangalist/delete/{}.xml", id)),
                }

                Ok(mal.client
                    .delete(url)
                    .with_auth(mal)
                    .send()?
                    .verify_status()?)
            }
            VerifyCredentials => {
                url.set_path("/api/account/verify_credentials.xml");
                Ok(mal.client.get(url).with_auth(mal).send()?)
            }
        }
    }
}

trait RequestExt {
    fn with_auth(&mut self, mal: &MAL) -> &mut RequestBuilder;
    fn with_body(&mut self, body: &str) -> &mut RequestBuilder;
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
}

trait ResponseExt {
    fn verify_status(self) -> Result<Response, BadResponse>;
}

#[derive(Fail, Debug)]
#[fail(display = "received bad response from MAL: {} {}", _0, _1)]
pub struct BadResponse(pub u16, pub String);

impl ResponseExt for Response {
    fn verify_status(self) -> Result<Response, BadResponse> {
        match self.status() {
            StatusCode::Ok | StatusCode::Created => Ok(self),
            status => {
                let reason = status.canonical_reason().unwrap_or("Unknown Error").into();
                Err(BadResponse(status.as_u16(), reason))
            }
        }
    }
}
