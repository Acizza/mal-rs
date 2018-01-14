//! The purpose of this library is to provide high-level access to the MyAnimeList API.
//! It allows you to search for anime / manga on MyAnimeList, as well as add / update / delete anime from a user's list.
//! 
//! # Examples
//! 
//! ```no_run
//! use mal::{MAL, AnimeInfo};
//! use mal::list::List;
//! use mal::list::anime::{AnimeEntry, WatchStatus};
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
//! entry.set_watched_episodes(5).set_status(WatchStatus::Watching);
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
mod util;

extern crate chrono;
extern crate minidom;
extern crate reqwest;

#[cfg(feature = "anime-list")]
use list::anime::AnimeList;
#[cfg(feature = "manga-list")]
use list::manga::MangaList;

use chrono::NaiveDate;
use failure::{Error, ResultExt, SyncFailure};
use list::ListType;
use minidom::Element;
use request::{Request, RequestError};
use reqwest::StatusCode;
use std::convert::Into;

/// Used to interact with the MyAnimeList API with authorization being handled automatically.
#[derive(Debug)]
pub struct MAL {
    /// The user's name on MyAnimeList.
    pub username: String,
    /// The user's password on MyAnimeList.
    pub password: String,
    /// The client used to send requests to the API.
    pub client: reqwest::Client,
}

impl MAL {
    /// Creates a new instance of the MAL struct for interacting with the MyAnimeList API.
    ///
    /// If you only need to retrieve the entries from a user's anime / manga list, then you do not need to provide a valid password.
    #[inline]
    pub fn new<S: Into<String>>(username: S, password: S) -> MAL {
        MAL::with_client(username, password, reqwest::Client::new())
    }

    /// Creates a new instance of the MAL struct for interacting with the MyAnimeList API.
    ///
    /// If you only need to retrieve the entries from a user's anime / manga list, then you do not need to provide a valid password.
    #[inline]
    pub fn with_client<S: Into<String>>(username: S, password: S, client: reqwest::Client) -> MAL {
        MAL {
            username: username.into(),
            password: password.into(),
            client,
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
    ///
    /// assert!(found.len() > 0);
    /// ```
    #[inline]
    pub fn search_anime(&self, name: &str) -> Result<Vec<AnimeInfo>, Error> {
        self.search::<AnimeInfo>(name)
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
    ///
    /// assert!(found.len() > 0);
    /// ```
    #[inline]
    pub fn search_manga(&self, name: &str) -> Result<Vec<MangaInfo>, Error> {
        self.search::<MangaInfo>(name)
    }

    fn search<SI: SeriesInfo>(&self, name: &str) -> Result<Vec<SI>, Error> {
        let mut resp = match Request::Search(name, SI::list_type()).send(self) {
            Ok(resp) => resp,
            Err(RequestError::BadResponseCode(StatusCode::NoContent)) => {
                return Ok(Vec::new());
            },
            Err(err) => bail!(err),
        };

        let root: Element = resp.text()?.parse().map_err(SyncFailure::new)?;
        let mut entries = Vec::new();

        for child in root.children() {
            let entry = SI::parse_search_result(child)?;
            entries.push(entry);
        }

        Ok(entries)
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
    pub fn verify_credentials(&self) -> Result<bool, Error> {
        let resp = Request::VerifyCredentials.send(self)?;
        Ok(resp.status() == StatusCode::Ok)
    }

    /// Returns a new [AnimeList] instance to perform operations on the user's anime list.
    /// 
    /// [AnimeList]: ./list/anime/struct.AnimeList.html
    #[cfg(feature = "anime-list")]
    #[inline]
    pub fn anime_list(&self) -> AnimeList {
        AnimeList::new(self)
    }

    /// Returns a new [MangaList] instance to perform operations on the user's manga list.
    /// 
    /// [MangaList]: ./list/manga/struct.MangaList.html
    #[cfg(feature = "manga-list")]
    #[inline]
    pub fn manga_list(&self) -> MangaList {
        MangaList::new(self)
    }
}

/// Represents series information for an anime or manga series.
pub trait SeriesInfo where Self: Sized {
    #[doc(hidden)]
    fn parse_search_result(xml_elem: &Element) -> Result<Self, Error>;

    #[doc(hidden)]
    fn list_type() -> ListType;
}

/// Represents basic information of an anime series on MyAnimeList.
#[derive(Debug, Clone)]
pub struct AnimeInfo {
    /// The ID of the anime series.
    pub id: u32,
    /// The title of the anime series.
    pub title: String,
    /// The alternative titles for the series.
    pub synonyms: Vec<String>,
    /// The number of episodes in the anime series.
    pub episodes: u32,
    /// The date the series started airing.
    pub start_date: Option<NaiveDate>,
    /// The date the series finished airing.
    pub end_date: Option<NaiveDate>,
    /// The URL to the cover image of the series.
    pub image_url: String,
}

impl SeriesInfo for AnimeInfo {
    #[doc(hidden)]
    fn parse_search_result(xml_elem: &Element) -> Result<AnimeInfo, Error> {
        let get_child = |name| {
            util::get_xml_child_text(xml_elem, name)
                .context("failed to parse MAL response")
        };

        let entry = AnimeInfo {
            id: get_child("id")?.parse()?,
            title: get_child("title")?,
            synonyms: util::split_into_vec(&get_child("synonyms")?, "; "),
            episodes: get_child("episodes")?.parse()?,
            start_date: util::parse_str_date(&get_child("start_date")?),
            end_date: util::parse_str_date(&get_child("end_date")?),
            image_url: get_child("image")?,
        };

        Ok(entry)
    }

    #[doc(hidden)]
    fn list_type() -> ListType {
        ListType::Anime
    }
}

impl PartialEq for AnimeInfo {
    #[inline]
    fn eq(&self, other: &AnimeInfo) -> bool {
        self.id == other.id
    }
}

/// Represents basic information of a manga series on MyAnimeList.
#[derive(Debug, Clone)]
pub struct MangaInfo {
    /// The ID of the manga series.
    pub id: u32,
    /// The title of the anime series.
    pub title: String,
    /// The alternative titles for the series.
    pub synonyms: Vec<String>,
    /// The number of chapters in the manga series.
    pub chapters: u32,
    /// The number of volumes in the manga series.
    pub volumes: u32,
    /// The date the series started airing.
    pub start_date: Option<NaiveDate>,
    /// The date the series finished airing.
    pub end_date: Option<NaiveDate>,
    /// The URL to the cover image of the series.
    pub image_url: String,
}

impl SeriesInfo for MangaInfo {
    #[doc(hidden)]
    fn parse_search_result(xml_elem: &Element) -> Result<MangaInfo, Error> {
        let get_child = |name| {
            util::get_xml_child_text(xml_elem, name)
                .context("failed to parse MAL response")
        };

        let entry = MangaInfo {
            id: get_child("id")?.parse()?,
            title: get_child("title")?,
            synonyms: util::split_into_vec(&get_child("synonyms")?, "; "),
            chapters: get_child("chapters")?.parse()?,
            volumes: get_child("volumes")?.parse()?,
            start_date: util::parse_str_date(&get_child("start_date")?),
            end_date: util::parse_str_date(&get_child("end_date")?),
            image_url: get_child("image")?,
        };

        Ok(entry)
    }

    #[doc(hidden)]
    fn list_type() -> ListType {
        ListType::Manga
    }
}

impl PartialEq for MangaInfo {
    #[inline]
    fn eq(&self, other: &MangaInfo) -> bool {
        self.id == other.id
    }
}
