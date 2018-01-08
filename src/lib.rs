//! The purpose of this library is to provide high-level access to the MyAnimeList API.
//! It allows you to search for anime / manga on MyAnimeList, as well as add / update / delete anime from a user's list.
//! 
//! # Examples
//! 
//! ```no_run
//! use mal::{MAL, SeriesInfo};
//! use mal::list::{AnimeList, ListEntry, Status};
//! 
//! // Create a new MAL instance
//! let mal = MAL::new("username", "password");
//! 
//! // Search for "Toradora" on MyAnimeList
//! let mut search_results = mal.search("Toradora").unwrap();
//! 
//! // Use the first result's info
//! let toradora_info = search_results.swap_remove(0);
//! 
//! // Create a new AnimeList instance
//! let anime_list = AnimeList::new(&mal);
//! 
//! // Create a new anime list entry with Toradora's info
//! let mut entry = ListEntry::new(toradora_info);
//! 
//! // Set the entry's watched episodes to 5 and status to watching
//! entry.set_watched_episodes(5).set_status(Status::Watching);
//! 
//! // Add the entry to the user's anime list
//! anime_list.add(&entry).unwrap();
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

use chrono::NaiveDate;
use failure::{Error, ResultExt, SyncFailure};
use minidom::Element;
use request::RequestURL;
use reqwest::StatusCode;
use std::convert::Into;

/// Represents basic information of an anime series on MyAnimeList.
#[derive(Debug, Clone)]
pub struct SeriesInfo {
    /// The ID of the anime series.
    pub id: u32,
    /// The title of the anime series.
    pub title: String,
    /// The number of episodes in the anime series.
    pub episodes: u32,
    /// The date the series started airing.
    pub start_date: Option<NaiveDate>,
    /// The date the series finished airing.
    pub end_date: Option<NaiveDate>,
}

impl PartialEq for SeriesInfo {
    #[inline]
    fn eq(&self, other: &SeriesInfo) -> bool {
        self.id == other.id
    }
}

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
    /// If you only need to retrieve the user's anime list, then you do not need to provide a valid password.
    #[inline]
    pub fn new<S: Into<String>>(username: S, password: S) -> MAL {
        MAL::with_client(username, password, reqwest::Client::new())
    }

    /// Creates a new instance of the MAL struct for interacting with the MyAnimeList API.
    ///
    /// If you only need to retrieve the user's anime list, then you do not need to provide a valid password.
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
    /// let found = mal.search("Cowboy Bebop").unwrap();
    ///
    /// assert!(found.len() > 0);
    /// ```
    pub fn search(&self, name: &str) -> Result<Vec<SeriesInfo>, Error> {
        let mut resp = request::auth_get(self, RequestURL::Search(name))?;

        if resp.status() == StatusCode::NoContent {
            return Ok(Vec::new());
        }

        let root: Element = resp.text()?.parse().map_err(SyncFailure::new)?;

        let mut entries = Vec::new();

        for child in root.children() {
            let get_child = |name| {
                util::get_xml_child_text(child, name)
                    .context("failed to parse MAL response")
            };

            let entry = SeriesInfo {
                id: get_child("id")?.parse()?,
                title: get_child("title")?,
                episodes: get_child("episodes")?.parse()?,
                start_date: util::parse_str_date(&get_child("start_date")?),
                end_date: util::parse_str_date(&get_child("end_date")?),
            };

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
        let resp = request::auth_get(self, RequestURL::VerifyCredentials)?;
        Ok(resp.status() == StatusCode::Ok)
    }
}
