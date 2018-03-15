//! The purpose of this library is to provide high-level access to the MyAnimeList API.
//! It allows you to add, update, delete, read, and search for anime / manga from a user's list,
//! as well as verify user credentials.
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
//! Adding an anime to a user's list:
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
//! let mut search_results = mal.anime_list().search_for("Toradora").unwrap();
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
//!
//! Updating a manga on a user's list by its ID:
//!
//! ```no_run
//! use mal::MAL;
//! use mal::list::Status;
//! use mal::list::manga::MangaValues;
//!
//! // Create a new MAL instance
//! let mal = MAL::new("username", "password");
//!
//! // Create new entry values
//! let mut values = MangaValues::new();
//!
//! // Set the number of read chapters to 25, read volumes to 2, score to 10, and status to completed
//! values.set_read_chapters(25)
//!       .set_read_volumes(2)
//!       .set_score(10)
//!       .set_status(Status::Completed);
//!
//! // Update the entry with an id of 2 (Berserk) on the user's manga list with the specified values
//! mal.manga_list().update_id(2, &mut values).unwrap();
//! ```
//!
//! Retrieving an anime off of a user's list and updating it:
//!
//! ```no_run
//! use mal::MAL;
//! use mal::list::Status;
//!
//! // Create a new MAL instance
//! let mal = MAL::new("username", "password");
//!
//! // Read the user's anime list
//! let list = mal.anime_list().read().unwrap();
//!
//! // Find the first series on the user's list that's being watched
//! let mut entry = list.entries.into_iter().find(|e| {
//!     e.values.status() == Status::WatchingOrReading
//! }).unwrap();
//!
//! // Set the entrie's watched episodes to its total episodes, its score to 10, and status to completed
//! entry.values
//!      .set_watched_episodes(entry.series_info.episodes)
//!      .set_score(10)
//!      .set_status(Status::Completed);
//!
//! // Update the entry on the user's anime list with the new values
//! mal.anime_list().update(&mut entry).unwrap();
//! ```

#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;

pub mod error;
pub mod list;

mod request;

extern crate chrono;
extern crate minidom;
extern crate reqwest;

#[cfg(feature = "anime")]
use list::anime::AnimeEntry;
#[cfg(feature = "manga")]
use list::manga::MangaEntry;

use error::{MALError, RequestError};
use list::{List, SeriesInfo};
use request::Request;
use reqwest::StatusCode;
use std::borrow::Cow;
use std::convert::Into;
use std::fmt::{self, Debug};

/// Used to interact with the MyAnimeList API with authorization being handled automatically.
#[derive(Clone)]
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
    /// and that will live as long as [`MAL`], then you should call [`with_client`] instead
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
    where
        S: Into<String>,
    {
        MAL {
            username: username.into(),
            password: password.into(),
            client,
        }
    }

    /// Returns a new [`List`] instance to perform anime list operations.
    ///
    /// [`List`]: ./list/struct.List.html
    #[cfg(feature = "anime")]
    #[inline]
    pub fn anime_list(&self) -> List<AnimeEntry> {
        List::<AnimeEntry>::new(self)
    }

    /// Returns a new [`List`] instance to perform manga list operations.
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

// Automatically deriving Debug will display the plain-text password
impl<'a> Debug for MAL<'a> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "MAL {{ username: {:?}, client: {:?} }}",
            self.username, self.client
        )
    }
}

impl<'a> PartialEq for MAL<'a> {
    #[inline]
    fn eq(&self, other: &MAL<'a>) -> bool {
        self.username == other.username && self.password == other.password
    }
}
