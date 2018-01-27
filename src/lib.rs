//! The purpose of this library is to provide high-level access to the MyAnimeList API.
//! It allows you to search for anime / manga, as well as add, update, delete, and read anime / manga from a user's list.
//! 
//! All operations are centered around the [`MAL`] struct, as it stores the user credentials
//! required to perform most operations on the API.
//! 
//! Please keep in mind that the API is rate limited to around ~5 requests per minute.
//! If you send too many requests, the caller's IP will be banned for ~1-2 hours and all
//! requests will return a 403 (Forbidden) status code.
//! 
//! [`MAL`]: ./struct.MAL.html
//! 
//! # Examples
//! 
//! ```no_run
//! use mal::MAL;
//! use mal::list::Status;
//! use mal::list::anime::AnimeEntry;
//! 
//! // Create a new MAL instance
//! let mal = MAL::new("username", "password");
//! 
//! // Search for "Toradora" on MyAnimeList
//! let mut search_results = mal.search_anime("Toradora").unwrap();
//! 
//! // Use the first result's info
//! let toradora_info = search_results.swap_remove(0);
//! 
//! // Create a new anime list entry with Toradora's info
//! let mut entry = AnimeEntry::new(toradora_info);
//! 
//! // Set the entry's watched episodes to 5 and status to watching
//! entry.values
//!      .set_watched_episodes(5)
//!      .set_status(Status::WatchingOrReading);
//! 
//! // Add the entry to the user's anime list
//! mal.anime_list().add(&mut entry).unwrap();
//! ```

#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;

pub mod list;

mod request;

extern crate chrono;
extern crate minidom;
extern crate reqwest;

#[cfg(feature = "anime")]
use list::anime::{AnimeEntry, AnimeInfo};
#[cfg(feature = "manga")]
use list::manga::{MangaEntry, MangaInfo};

use failure::SyncFailure;
use list::{List, SeriesInfo};
use minidom::Element;
use request::{Request, RequestError};
use reqwest::StatusCode;
use std::borrow::Cow;
use std::convert::Into;

#[derive(Fail, Debug)]
pub enum MALError {
    #[fail(display = "{}", _0)]
    Request(#[cause] ::request::RequestError),

    #[fail(display = "internal error: {}", _0)]
    Internal(::failure::Error),
}

/// Used to interact with the MyAnimeList API with authorization being handled automatically.
#[derive(Debug)]
pub struct MAL<'a> {
    /// The user's name on MyAnimeList.
    pub username: String,
    /// The user's password on MyAnimeList.
    pub password: String,
    /// The client used to send requests to the API.
    pub client: Cow<'a, reqwest::Client>,
}

impl<'a> MAL<'a> {
    /// Creates a new instance of the MAL struct for interacting with the MyAnimeList API.
    /// If you only need to retrieve the entries from a user's list, then you do not need to
    /// provide a valid password.
    /// 
    /// This function will create a new reqwest [`Client`] to send requests to MyAnimeList.
    /// If you already have a [`Client`] that you only need to make synchronous requests with
    /// and will that live as long as [`MAL`], then you should call [`with_client`] instead
    /// with `Cow::Borrowed`.
    /// 
    /// [`Client`]: ./../reqwest/struct.Client.html
    /// [`MAL`]: ./struct.MAL.html
    /// [`with_client`]: #method.with_client
    #[inline]
    pub fn new<S: Into<String>>(username: S, password: S) -> MAL<'a> {
        MAL::with_client(username, password, Cow::Owned(reqwest::Client::new()))
    }

    /// Creates a new instance of the MAL struct for interacting with the MyAnimeList API.
    /// If you only need to retrieve the entries from a user's list, then you do not need to
    /// provide a valid password.
    #[inline]
    pub fn with_client<S>(username: S, password: S, client: Cow<'a, reqwest::Client>) -> MAL<'a>
        where S: Into<String> {
        MAL {
            username: username.into(),
            password: password.into(),
            client: client.into(),
        }
    }

    /// Searches MyAnimeList for an anime and returns all found results.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mal::MAL;
    ///
    /// let mal = MAL::new("username", "password");
    /// let found = mal.search_anime("Cowboy Bebop").unwrap();
    /// ```
    #[cfg(feature = "anime")]
    #[inline]
    pub fn search_anime<S>(&self, name: S) -> Result<Vec<AnimeInfo>, MALError>
        where S: AsRef<str> {
        self.search::<AnimeInfo, S>(name)
    }

    /// Searches MyAnimeList for a manga and returns all found results.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mal::MAL;
    ///
    /// let mal = MAL::new("username", "password");
    /// let found = mal.search_manga("Bleach").unwrap();
    /// ```
    #[cfg(feature = "manga")]
    #[inline]
    pub fn search_manga<S>(&self, name: S) -> Result<Vec<MangaInfo>, MALError>
        where S: AsRef<str> {
        self.search::<MangaInfo, S>(name)
    }

    /// Searches MyAnimeList for the type of series specified by the `I` type parameter
    /// and returns all found results.
    /// 
    /// If you aren't writing generic code over the [`SeriesInfo`] trait, you should
    /// ideally use the [`search_anime`] and [`search_manga`] methods instead to
    /// avoid ever accidentally searching for the wrong type of series.
    /// 
    /// [`SeriesInfo`]: ./list/trait.SeriesInfo.html
    /// [`search_anime`]: #method.search_anime
    /// [`search_manga`]: #method.search_manga
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mal::MAL;
    /// use mal::list::anime::AnimeInfo;
    /// 
    /// let mal = MAL::new("username", "password");
    /// let found: Vec<AnimeInfo> = mal.search("Cowboy Bebop").unwrap();
    /// ```
    pub fn search<I, S>(&self, name: S) -> Result<Vec<I>, MALError>
        where I: SeriesInfo, S: AsRef<str> {
        let resp = {
            let result = Request::Search(name.as_ref(), I::list_type()).send(self);

            match result {
                Ok(resp) => resp,
                Err(RequestError::BadResponseCode(StatusCode::NoContent)) => {
                    return Ok(Vec::new());
                },
                Err(err) => return Err(MALError::Request(err)),
            }
        };

        let root: Element = resp
            .parse()
            .map_err(|e| MALError::Internal(SyncFailure::new(e).into()))?;

        let mut entries = Vec::new();

        for child in root.children() {
            let entry = I::parse_search_result(child)
                .map_err(MALError::Internal)?;

            entries.push(entry);
        }

        Ok(entries)
    }

    /// Returns a new [`List`] instance that performs operations on the user's anime list.
    /// 
    /// [`List`]: ./list/struct.List.html
    #[cfg(feature = "anime")]
    #[inline]
    pub fn anime_list(&self) -> List<AnimeEntry> {
        List::<AnimeEntry>::new(self)
    }

    /// Returns a new [`List`] instance that performs operations on the user's manga list.
    /// 
    /// [`List`]: ./list/struct.List.html
    #[cfg(feature = "manga")]
    #[inline]
    pub fn manga_list(&self) -> List<MangaEntry> {
        List::<MangaEntry>::new(self)
    }

    /// Returns true if the provided account credentials are correct.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mal::MAL;
    /// 
    /// // Create a new MAL instance
    /// let mal = MAL::new("username", "password");
    /// 
    /// // Verify that the username and password are valid
    /// let valid = mal.verify_credentials().unwrap();
    /// 
    /// assert_eq!(valid, false);
    /// ```
    #[inline]
    pub fn verify_credentials(&self) -> Result<bool, MALError> {
        match Request::VerifyCredentials.send(self) {
            Ok(_) => Ok(true),
            Err(RequestError::BadResponseCode(StatusCode::Unauthorized)) => Ok(false),
            Err(err) => Err(MALError::Request(err)),
        }
    }
}
