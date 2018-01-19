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
//! entry.values
//!      .set_watched_episodes(5)
//!      .set_status(WatchStatus::Watching);
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
use list::anime::AnimeEntry;
#[cfg(feature = "manga-list")]
use list::manga::MangaEntry;

use chrono::NaiveDate;
use failure::{Error, SyncFailure};
use list::List;
use minidom::Element;
use request::{ListType, Request, RequestError};
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
    /// If you only need to retrieve the entries from a user's list, then you do not need to provide a valid password.
    #[inline]
    pub fn new<S: Into<String>>(username: S, password: S) -> MAL {
        MAL::with_client(username, password, reqwest::Client::new())
    }

    /// Creates a new instance of the MAL struct for interacting with the MyAnimeList API.
    ///
    /// If you only need to retrieve the entries from a user's list, then you do not need to provide a valid password.
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
        match Request::VerifyCredentials.send(self) {
            Ok(_) => Ok(true),
            Err(RequestError::BadResponseCode(StatusCode::Unauthorized)) => Ok(false),
            Err(err) => bail!(err),
        }
    }

    /// Returns a new [`List`] instance that performs operations on the user's anime list.
    /// 
    /// [`List`]: ./list/struct.List.html
    #[cfg(feature = "anime-list")]
    #[inline]
    pub fn anime_list(&self) -> List<AnimeEntry> {
        List::<AnimeEntry>::new(self)
    }

    /// Returns a new [`List`] instance that performs operations on the user's manga list.
    /// 
    /// [`List`]: ./list/struct.List.html
    #[cfg(feature = "manga-list")]
    #[inline]
    pub fn manga_list(&self) -> List<MangaEntry> {
        List::<MangaEntry>::new(self)
    }
}

/// Represents series information for an anime or manga series.
pub trait SeriesInfo where Self: Sized {
    #[doc(hidden)]
    fn parse_search_result(xml_elem: &Element) -> Result<Self, Error>;

    #[doc(hidden)]
    fn list_type() -> ListType;
}

#[derive(Fail, Debug)]
pub enum SeriesInfoError {
    #[fail(display = "no series type named \"{}\" found", _0)]
    UnknownSeriesType(String),
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
    /// The type of series that this is.
    pub series_type: AnimeType,
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
        let get_child = |name| util::get_xml_child_text(xml_elem, name);

        let entry = AnimeInfo {
            id: get_child("id")?.parse()?,
            title: get_child("title")?,
            synonyms: util::split_into_vec(&get_child("synonyms")?, "; "),
            episodes: get_child("episodes")?.parse()?,
            series_type: {
                let s_type = get_child("type")?;

                AnimeType::from_str(&s_type)
                    .ok_or_else(|| SeriesInfoError::UnknownSeriesType(s_type))?
            },
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

/// Represents an anime series type.
#[derive(Debug, Clone, PartialEq)]
pub enum AnimeType {
    /// A series that has aired on TV.
    TV = 1,
    /// A series that has never aired on TV.
    OVA,
    /// A series depicted in the form of a movie.
    Movie,
    /// An extra set of episodes from a series that are usually self-contained.
    Special,
    /// A series that has only been presented on the internet.
    ONA,
}

impl AnimeType {
    #[cfg(feature = "anime-list")]
    pub(crate) fn from_i32(value: i32) -> Option<AnimeType> {
        match value {
            1 => Some(AnimeType::TV),
            2 => Some(AnimeType::OVA),
            3 => Some(AnimeType::Movie),
            4 => Some(AnimeType::Special),
            5 => Some(AnimeType::ONA),
            _ => None,
        }
    }

    pub(crate) fn from_str<S: AsRef<str>>(input: S) -> Option<AnimeType> {
        let lowered = input
            .as_ref()
            .to_ascii_lowercase();

        match lowered.as_str() {
            "tv" => Some(AnimeType::TV),
            "ova" => Some(AnimeType::OVA),
            "movie" => Some(AnimeType::Movie),
            "special" => Some(AnimeType::Special),
            "ona" => Some(AnimeType::ONA),
            _ => None,
        }
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
    /// The type of series that this is.
    pub series_type: MangaType,
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
        let get_child = |name| util::get_xml_child_text(xml_elem, name);

        let entry = MangaInfo {
            id: get_child("id")?.parse()?,
            title: get_child("title")?,
            synonyms: util::split_into_vec(&get_child("synonyms")?, "; "),
            series_type: {
                let s_type = get_child("type")?;

                MangaType::from_str(&s_type)
                    .ok_or_else(|| SeriesInfoError::UnknownSeriesType(s_type))?
            },
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

/// Represents a manga series type.
#[derive(Debug, Clone, PartialEq)]
pub enum MangaType {
    Manga = 1,
    Novel,
    /// A manga series with a single chapter.
    OneShot,
    /// A self-published manga series.
    Doujinshi,
    /// A South Korean manga series.
    Manhwa,
    /// A Taiwanese manga series.
    Manhua,
}

impl MangaType {
    #[cfg(feature = "manga-list")]
    pub(crate) fn from_i32(value: i32) -> Option<MangaType> {
        match value {
            1 => Some(MangaType::Manga),
            2 => Some(MangaType::Novel),
            3 => Some(MangaType::OneShot),
            4 => Some(MangaType::Doujinshi),
            5 => Some(MangaType::Manhwa),
            6 => Some(MangaType::Manhua),
            _ => None,
        }
    }

    pub(crate) fn from_str<S: AsRef<str>>(input: S) -> Option<MangaType> {
        let lowered = input
            .as_ref()
            .to_ascii_lowercase();

        match lowered.as_str() {
            "manga" => Some(MangaType::Manga),
            "novel" => Some(MangaType::Novel),
            "one-shot" => Some(MangaType::OneShot),
            "doujinshi" => Some(MangaType::Doujinshi),
            "manhwa" => Some(MangaType::Manhwa),
            "manhua" => Some(MangaType::Manhua),
            _ => None,
        }
    }
}
