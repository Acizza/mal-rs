use failure::Error;
use list::ListType;
use MAL;
use reqwest::{Client, RequestBuilder, Response, StatusCode, Url};
use reqwest::header::{ContentType, Headers};

pub type ID = u32;

#[derive(Debug)]
pub enum RequestURL<'a> {
    List(&'a str, ListType),
    Search(&'a str, ListType),
    Add(ID, ListType),
    Update(ID, ListType),
    Delete(ID, ListType),
    VerifyCredentials,
}

impl<'a> RequestURL<'a> {
    pub const BASE_URL: &'static str = "https://myanimelist.net";
}

impl<'a> Into<Url> for RequestURL<'a> {
    fn into(self) -> Url {
        lazy_static! {
            static ref BASE_URL: Url = Url::parse(RequestURL::BASE_URL).unwrap();
        }

        let mut url = BASE_URL.clone();

        match self {
            RequestURL::List(uname, list_type) => {
                url.set_path("/malappinfo.php");

                let query = match list_type {
                    ListType::Anime => "anime",
                    ListType::Manga => "manga",
                };

                url.query_pairs_mut()
                    .append_pair("u", uname)
                    .append_pair("status", "all")
                    .append_pair("type", query);
            }
            RequestURL::Search(name, ListType::Anime) => {
                url.set_path("/api/anime/search.xml");
                url.query_pairs_mut().append_pair("q", name);
            }
            RequestURL::Search(name, ListType::Manga) => {
                url.set_path("/api/manga/search.xml");
                url.query_pairs_mut().append_pair("q", name);
            }
            RequestURL::Add(id, ListType::Anime) => {
                url.set_path(&format!("/api/animelist/add/{}.xml", id));
            }
            RequestURL::Add(id, ListType::Manga) => {
                url.set_path(&format!("/api/mangalist/add/{}.xml", id));
            }
            RequestURL::Update(id, ListType::Anime) => {
                url.set_path(&format!("/api/animelist/update/{}.xml", id));
            }
            RequestURL::Update(id, ListType::Manga) => {
                url.set_path(&format!("/api/mangalist/update/{}.xml", id));
            }
            RequestURL::Delete(id, ListType::Anime) => {
                url.set_path(&format!("/api/animelist/delete/{}.xml", id));
            }
            RequestURL::Delete(id, ListType::Manga) => {
                url.set_path(&format!("/api/mangalist/delete/{}.xml", id));
            }
            RequestURL::VerifyCredentials => {
                url.set_path("/api/account/verify_credentials.xml");
            }
        }

        url
    }
}

pub fn get(client: &Client, req_type: RequestURL) -> Result<Response, Error> {
    let url: Url = req_type.into();
    Ok(client.get(url).send()?)
}

pub fn get_verify(client: &Client, req_type: RequestURL) -> Result<Response, Error> {
    let resp = get(client, req_type)?;
    verify_good_response(&resp)?;

    Ok(resp)
}

pub fn auth_get(mal: &MAL, req_type: RequestURL) -> Result<Response, Error> {
    let url: Url = req_type.into();
    send_auth_req(mal, &mut mal.client.get(url))
}

pub fn auth_post(mal: &MAL, req_type: RequestURL, body: &str) -> Result<Response, Error> {
    let mut headers = Headers::new();
    headers.set(ContentType::form_url_encoded());

    let url: Url = req_type.into();

    send_auth_req(
        mal,
        mal.client
            .post(url)
            .body(format!("data={}", body))
            .headers(headers),
    )
}

pub fn auth_post_verify(mal: &MAL, req_type: RequestURL, body: &str) -> Result<Response, Error> {
    let resp = auth_post(mal, req_type, body)?;
    verify_good_response(&resp)?;

    Ok(resp)
}

pub fn auth_delete(mal: &MAL, req_type: RequestURL) -> Result<Response, Error> {
    let url: Url = req_type.into();
    send_auth_req(mal, &mut mal.client.delete(url))
}

pub fn auth_delete_verify(mal: &MAL, req_type: RequestURL) -> Result<Response, Error> {
    let resp = auth_delete(mal, req_type)?;
    verify_good_response(&resp)?;

    Ok(resp)
}

fn send_auth_req(mal: &MAL, req: &mut RequestBuilder) -> Result<Response, Error> {
    let resp = req.basic_auth(mal.username.clone(), Some(mal.password.clone()))
        .send()?;

    Ok(resp)
}

#[derive(Fail, Debug)]
#[fail(display = "received bad response from MAL: {} {}", _0, _1)]
pub struct BadResponse(pub u16, pub String);

pub fn verify_good_response(resp: &Response) -> Result<(), BadResponse> {
    match resp.status() {
        StatusCode::Ok | StatusCode::Created => Ok(()),
        status => {
            let reason = status.canonical_reason().unwrap_or("Unknown Error").into();
            Err(BadResponse(status.as_u16(), reason))
        }
    }
}
