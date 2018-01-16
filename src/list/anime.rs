//! This module handles adding / updating / removing anime to a user's anime list.

use AnimeInfo;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use failure::{Error, SyncFailure};
use MAL;
use minidom::Element;
use std::fmt::{self, Display};
use super::{ChangeTracker, EntryValues, List, ListEntry, ListType};
use util;

/// Used to perform operations on a user's anime list.
///
/// Note that since the `AnimeList` struct stores a reference to a [MAL] instance,
/// the [MAL] instance must live as long as the `AnimeList`.
///
/// [MAL]: ../../struct.MAL.html
#[derive(Debug, Copy, Clone)]
pub struct AnimeList<'a> {
    /// A reference to the MyAnimeList client used to add and update anime on a user's list.
    pub mal: &'a MAL,
}

impl<'a> AnimeList<'a> {
    /// Creates a new instance of the `AnimeList` struct and stores the provided [MAL] reference
    /// so authorization can be handled automatically.
    ///
    /// [MAL]: ../../struct.MAL.html
    #[inline]
    pub fn new(mal: &'a MAL) -> AnimeList<'a> {
        AnimeList { mal }
    }
}

impl<'a> List for AnimeList<'a> {
    type Entry = AnimeEntry;
    type EntryValues = AnimeValues;

    #[inline]
    fn list_type() -> ListType {
        ListType::Anime
    }

    #[inline]
    fn mal(&self) -> &MAL {
        self.mal
    }
}

#[derive(Fail, Debug)]
pub enum AnimeEntryError {
    #[fail(display = "{} is not a known watch status", _0)] UnknownWatchStatus(i32),
}

/// Represents information about an anime series on a user's list.
#[derive(Debug, Clone)]
pub struct AnimeEntry {
    /// The general series information.
    pub series_info: AnimeInfo,
    /// The last time the series was updated.
    pub last_updated_time: DateTime<Utc>,
    /// Contains values that can be set / updated on a user's list.
    pub values: AnimeValues,
}

impl AnimeEntry {
    /// Creates a new `AnimeEntry` instance with [AnimeInfo] obtained from [MAL].
    ///
    /// [MAL]: ../../struct.MAL.html
    /// [AnimeInfo]: ../../struct.AnimeInfo.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use mal::MAL;
    /// use mal::list::anime::AnimeEntry;
    ///
    /// // Create a new MAL instance
    /// let mal = MAL::new("username", "password");
    ///
    /// // Search for Toradora on MAL
    /// let mut results = mal.search_anime("Toradora").unwrap();
    ///
    /// // Select the first result
    /// let toradora_info = results.swap_remove(0);
    ///
    /// // Create a new AnimeEntry that represents Toradora with default values
    /// let entry = AnimeEntry::new(toradora_info);
    /// ```
    #[inline]
    pub fn new(info: AnimeInfo) -> AnimeEntry {
        AnimeEntry {
            series_info: info,
            last_updated_time: Utc::now(),
            values: AnimeValues::new(),
        }
    }
}

impl ListEntry<AnimeValues> for AnimeEntry {
    #[doc(hidden)]
    fn parse(xml_elem: &Element) -> Result<AnimeEntry, Error> {
        let get_child = |name| util::get_xml_child_text(xml_elem, name);

        let info = AnimeInfo {
            id: get_child("series_animedb_id")?.parse()?,
            title: get_child("series_title")?,
            synonyms: util::split_into_vec(&get_child("series_synonyms")?, "; "),
            episodes: get_child("series_episodes")?.parse()?,
            start_date: util::parse_str_date(&get_child("series_start")?),
            end_date: util::parse_str_date(&get_child("series_end")?),
            image_url: get_child("series_image")?,
        };

        let entry = AnimeEntry {
            series_info: info,
            last_updated_time: Utc.timestamp(get_child("my_last_updated")?.parse()?, 0),
            values: AnimeValues::parse(xml_elem)?,
        };

        Ok(entry)
    }

    #[doc(hidden)]
    #[inline]
    fn values_mut(&mut self) -> &mut AnimeValues {
        &mut self.values
    }

    #[doc(hidden)]
    #[inline]
    fn set_last_updated_time(&mut self) {
        self.last_updated_time = Utc::now();
    }

    #[doc(hidden)]
    #[inline]
    fn id(&self) -> u32 {
        self.series_info.id
    }
}

impl PartialEq for AnimeEntry {
    #[inline]
    fn eq(&self, other: &AnimeEntry) -> bool {
        self.series_info == other.series_info
    }
}

/// Contains values that can set / updated on a user's list.
///
/// # Examples
///
/// ```
/// use mal::list::anime::{AnimeValues, WatchStatus};
///
/// let mut values = AnimeValues::new();
///
/// values.set_watched_episodes(5)
///       .set_status(WatchStatus::Watching)
///       .set_score(7);
///
/// assert_eq!(values.watched_episodes(), 5);
/// assert_eq!(values.status(), WatchStatus::Watching);
/// assert_eq!(values.score(), 7);
/// ```
#[derive(Debug, Clone)]
pub struct AnimeValues {
    watched_episodes: ChangeTracker<u32>,
    start_date: ChangeTracker<Option<NaiveDate>>,
    finish_date: ChangeTracker<Option<NaiveDate>>,
    status: ChangeTracker<WatchStatus>,
    score: ChangeTracker<u8>,
    rewatching: ChangeTracker<bool>,
    tags: ChangeTracker<Vec<String>>,
}

impl AnimeValues {
    /// Creates a new `AnimeValues` instance with default values.
    #[inline]
    pub fn new() -> AnimeValues {
        AnimeValues {
            watched_episodes: 0.into(),
            start_date: None.into(),
            finish_date: None.into(),
            status: WatchStatus::default().into(),
            score: 0.into(),
            rewatching: false.into(),
            tags: Vec::new().into(),
        }
    }

    fn parse(xml_elem: &Element) -> Result<AnimeValues, Error> {
        let get_child = |name| util::get_xml_child_text(xml_elem, name);

        let values = AnimeValues {
            watched_episodes: get_child("my_watched_episodes")?.parse::<u32>()?.into(),
            start_date: util::parse_str_date(&get_child("my_start_date")?).into(),
            finish_date: util::parse_str_date(&get_child("my_finish_date")?).into(),
            status: {
                let status_num = get_child("my_status")?.parse()?;

                WatchStatus::from_i32(status_num)
                    .ok_or_else(|| AnimeEntryError::UnknownWatchStatus(status_num))?
                    .into()
            },
            score: get_child("my_score")?.parse::<u8>()?.into(),
            rewatching: {
                // The rewatching tag is sometimes blank for no apparent reason..
                get_child("my_rewatching")?
                    .parse::<u8>()
                    .map(|v| v == 1)
                    .unwrap_or(false)
                    .into()
            },
            tags: util::split_into_vec(&get_child("my_tags")?, ",").into(),
        };

        Ok(values)
    }

    /// Returns the number of episodes watched.
    #[inline]
    pub fn watched_episodes(&self) -> u32 {
        self.watched_episodes.value
    }

    /// Sets the watched episode count.
    #[inline]
    pub fn set_watched_episodes(&mut self, watched: u32) -> &mut AnimeValues {
        self.watched_episodes.set(watched);
        self
    }

    /// Returns the date the anime started being watched.
    #[inline]
    pub fn start_date(&self) -> Option<NaiveDate> {
        self.start_date.value
    }

    /// Sets the date the user started watching the anime.
    #[inline]
    pub fn set_start_date(&mut self, date: Option<NaiveDate>) -> &mut AnimeValues {
        self.start_date.set(date);
        self
    }

    /// Returns the date the anime finished being watched.
    #[inline]
    pub fn finish_date(&self) -> Option<NaiveDate> {
        self.finish_date.value
    }

    /// Sets the date the user finished watching the anime.
    #[inline]
    pub fn set_finish_date(&mut self, date: Option<NaiveDate>) -> &mut AnimeValues {
        self.finish_date.set(date);
        self
    }

    /// Returns the current watch status of the anime.
    #[inline]
    pub fn status(&self) -> WatchStatus {
        self.status.value
    }

    /// Sets the current watch status for the anime.
    #[inline]
    pub fn set_status(&mut self, status: WatchStatus) -> &mut AnimeValues {
        self.status.set(status);
        self
    }

    /// Returns the user's score of the anime.
    #[inline]
    pub fn score(&self) -> u8 {
        self.score.value
    }

    /// Sets the user's score for the anime.
    #[inline]
    pub fn set_score(&mut self, score: u8) -> &mut AnimeValues {
        self.score.set(score);
        self
    }

    /// Returns true if the anime is currently being rewatched.
    #[inline]
    pub fn rewatching(&self) -> bool {
        self.rewatching.value
    }

    /// Sets whether or not the user is currently rewatching the anime.
    #[inline]
    pub fn set_rewatching(&mut self, rewatching: bool) -> &mut AnimeValues {
        self.rewatching.set(rewatching);
        self
    }

    /// Returns the tags the user has set for the anime.
    #[inline]
    pub fn tags(&self) -> &Vec<String> {
        &self.tags.value
    }

    /// Returns a mutable reference to the tags the user has set for the anime.
    #[inline]
    pub fn tags_mut(&mut self) -> &mut Vec<String> {
        // If a mutable reference is being requested, then it's safe to assume the values
        // are going to be changed
        self.tags.changed = true;
        &mut self.tags.value
    }
}

impl EntryValues for AnimeValues {
    #[doc(hidden)]
    fn generate_xml(&self) -> Result<String, Error> {
        generate_response_xml!(self,
            watched_episodes(num): "episode" => num.to_string(),
            status(status): "status" => (*status as i32).to_string(),
            start_date(date): "date_start" => util::date_to_str(*date),
            finish_date(date): "date_finish" => util::date_to_str(*date),
            score(score): "score" => score.to_string(),
            rewatching(v): "enable_rewatching" => (*v as u8).to_string(),
            tags(t): "tags" => util::concat_by_delimeter(t, ',')
        )
    }

    #[doc(hidden)]
    #[inline]
    fn reset_changed_fields(&mut self) {
        reset_changed_fields!(
            self,
            watched_episodes,
            start_date,
            finish_date,
            status,
            score,
            rewatching,
            tags
        );
    }
}

/// Represents the watch status of an anime on a user's list.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum WatchStatus {
    Watching = 1,
    Completed,
    OnHold,
    Dropped,
    PlanToWatch = 6,
}

impl WatchStatus {
    /// Attempts to convert an i32 to a `WatchStatus`.
    ///
    /// Note that the i32 value of each `WatchStatus` variant is mapped
    /// to the one provided by the MyAnimeList API, so they do not increment naturally.
    ///
    /// # Example
    ///
    /// ```
    /// use mal::list::anime::WatchStatus;
    ///
    /// let status = WatchStatus::from_i32(1).unwrap();
    /// assert_eq!(status, WatchStatus::Watching);
    /// ```
    #[inline]
    pub fn from_i32(value: i32) -> Option<WatchStatus> {
        match value {
            1 => Some(WatchStatus::Watching),
            2 => Some(WatchStatus::Completed),
            3 => Some(WatchStatus::OnHold),
            4 => Some(WatchStatus::Dropped),
            6 => Some(WatchStatus::PlanToWatch),
            _ => None,
        }
    }
}

impl Default for WatchStatus {
    #[inline]
    fn default() -> Self {
        WatchStatus::PlanToWatch
    }
}

impl Display for WatchStatus {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            WatchStatus::Watching => write!(f, "watching"),
            WatchStatus::Completed => write!(f, "completed"),
            WatchStatus::OnHold => write!(f, "on hold"),
            WatchStatus::Dropped => write!(f, "dropped"),
            WatchStatus::PlanToWatch => write!(f, "plan to watch"),
        }
    }
}
