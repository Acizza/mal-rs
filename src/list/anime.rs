//! Contains the required data structures to search for anime on MyAnimeList and
//! perform operations on a user's anime list.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use failure::{Error, SyncFailure};
use minidom::Element;
use request::ListType;
use SeriesInfo;
use std::fmt::{self, Display};
use super::{ChangeTracker, EntryValues, ListEntry, UserInfo};
use util::{self, parse_xml_child};

#[derive(Fail, Debug)]
pub enum AnimeError {
    #[fail(display = "\"{}\" does not map to a known airing status", _0)]
    UnknownAirStatus(String),

    #[fail(display = "{} is not a known watch status", _0)]
    UnknownWatchStatus(i32),

    #[fail(display = "\"{}\" does not map to a known series type", _0)]
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
    /// The current airing status of the series.
    pub airing_status: AiringStatus,
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
    fn parse_search_result(xml: &Element) -> Result<AnimeInfo, Error> {
        let entry = AnimeInfo {
            id: parse_xml_child(xml, "id")?,
            title: parse_xml_child(xml, "title")?,
            synonyms: util::split_into_vec(&parse_xml_child::<String>(xml, "synonyms")?, "; "),
            episodes: parse_xml_child(xml, "episodes")?,
            airing_status: {
                let status = parse_xml_child(xml, "status")?;
                AiringStatus::from_str(&status).ok_or_else(|| AnimeError::UnknownAirStatus(status))?
            },
            series_type: {
                let s_type = parse_xml_child(xml, "type")?;
                AnimeType::from_str(&s_type).ok_or_else(|| AnimeError::UnknownSeriesType(s_type))?
            },
            start_date: util::parse_str_date(&parse_xml_child::<String>(xml, "start_date")?),
            end_date: util::parse_str_date(&parse_xml_child::<String>(xml, "end_date")?),
            image_url: parse_xml_child(xml, "image")?,
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

gen_list_field_enum!(AnimeType,
    ["A series that has aired on TV."]
    TV = [1, "tv"],

    ["A series that has never aired on TV."]
    OVA = [2, "ova"],

    ["A series depicted in the form of a movie."]
    Movie = [3, "movie"],

    ["An extra set of episodes from a series that are usually self-contained."]
    Special = [4, "special"],

    ["A series that has only been presented on the internet."]
    ONA = [5, "ona"],
);

gen_list_field_enum!(AiringStatus,
    ["A series that is currently airing."]
    Airing = [1, "currently airing"],

    ["A series that has finished airing."]
    FinishedAiring = [2, "finished airing"],

    ["A series that hasn't aired yet."]
    NotYetAired = [3, "not yet aired"],
);

/// Contains information about an anime series on a user's list.
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
    /// Creates a new `AnimeEntry` instance with [`AnimeInfo`] obtained from [`MAL`].
    ///
    /// [`MAL`]: ../../struct.MAL.html
    /// [`AnimeInfo`]: ../../struct.AnimeInfo.html
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

impl ListEntry for AnimeEntry {
    type Values = AnimeValues;
    type UserInfo = AnimeUserInfo;

    #[doc(hidden)]
    fn parse(xml: &Element) -> Result<AnimeEntry, Error> {
        let info = AnimeInfo {
            id: parse_xml_child(xml, "series_animedb_id")?,
            title: parse_xml_child(xml, "series_title")?,
            synonyms: {
                util::split_into_vec(&parse_xml_child::<String>(xml, "series_synonyms")?, "; ")
            },
            episodes: parse_xml_child(xml, "series_episodes")?,
            airing_status: {
                let status = parse_xml_child(xml, "series_status")?;

                AiringStatus::from_i32(status)
                    .ok_or_else(|| AnimeError::UnknownAirStatus(status.to_string()))?
            },
            series_type: {
                let s_type = parse_xml_child(xml, "series_type")?;

                AnimeType::from_i32(s_type)
                    .ok_or_else(|| AnimeError::UnknownSeriesType(s_type.to_string()))?
            },
            start_date: util::parse_str_date(&parse_xml_child::<String>(xml, "series_start")?),
            end_date: util::parse_str_date(&parse_xml_child::<String>(xml, "series_end")?),
            image_url: parse_xml_child(xml, "series_image")?,
        };

        let entry = AnimeEntry {
            series_info: info,
            last_updated_time: Utc.timestamp(parse_xml_child(xml, "my_last_updated")?, 0),
            values: AnimeValues::parse(xml)?,
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

    #[doc(hidden)]
    #[inline]
    fn list_type() -> ListType {
        ListType::Anime
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
#[derive(Debug, Default, Clone)]
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
        AnimeValues::default()
    }

    fn parse(xml: &Element) -> Result<AnimeValues, Error> {
        let values = AnimeValues {
            watched_episodes: parse_xml_child::<u32>(xml, "my_watched_episodes")?.into(),
            start_date: {
                util::parse_str_date(&parse_xml_child::<String>(xml, "my_start_date")?).into()
            },
            finish_date: {
                util::parse_str_date(&parse_xml_child::<String>(xml, "my_finish_date")?).into()
            },
            status: {
                let status_num = parse_xml_child(xml, "my_status")?;

                WatchStatus::from_i32(status_num)
                    .ok_or_else(|| AnimeError::UnknownWatchStatus(status_num))?
                    .into()
            },
            score: parse_xml_child::<u8>(xml, "my_score")?.into(),
            rewatching: {
                // The rewatching tag is sometimes blank for no apparent reason..
                parse_xml_child::<u8>(xml, "my_rewatching")
                    .map(|v| v == 1)
                    .unwrap_or(false)
                    .into()
            },
            tags: util::split_into_vec(&parse_xml_child::<String>(xml, "my_tags")?, ",").into(),
        };

        Ok(values)
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

impl_tracker_getset!(AnimeValues,
    [watched_episodes, set_watched_episodes, "number of watched episodes"]: u32,
    [start_date, set_start_date, "date the user started watching the series"]: Option<NaiveDate>,
    [finish_date, set_finish_date, "date the user finished watching the series"]: Option<NaiveDate>,
    [status, set_status, "current watch status of the series"]: WatchStatus,
    [score, set_score, "user's rating of the series"]: u8,
    [rewatching, set_rewatching, "current rewatch status of the series"]: bool,
);

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

/// Contains list statistics and user information.
#[derive(Debug, Clone)]
pub struct AnimeUserInfo {
    /// The user's ID.
    pub user_id: u32,
    /// The number of anime being watched.
    pub watching: u32,
    /// The number of anime that have been completed.
    pub completed: u32,
    /// The number of anime on hold.
    pub on_hold: u32,
    /// The number of anime dropped.
    pub dropped: u32,
    /// The number of anime that are planning to be watched.
    pub plan_to_watch: u32,
    /// The total days spent watching all of the anime on the user's list.
    pub days_spent_watching: f32,
}

impl UserInfo for AnimeUserInfo {
    #[doc(hidden)]
    fn parse(xml: &Element) -> Result<AnimeUserInfo, Error> {
        let info = AnimeUserInfo {
            user_id: parse_xml_child(xml, "user_id")?,
            watching: parse_xml_child(xml, "user_watching")?,
            completed: parse_xml_child(xml, "user_completed")?,
            on_hold: parse_xml_child(xml, "user_onhold")?,
            dropped: parse_xml_child(xml, "user_dropped")?,
            plan_to_watch: parse_xml_child(xml, "user_plantowatch")?,
            days_spent_watching: parse_xml_child(xml, "user_days_spent_watching")?,
        };

        Ok(info)
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
    /// # Example
    ///
    /// ```
    /// use mal::list::anime::WatchStatus;
    ///
    /// let status_watching = WatchStatus::from_i32(1).unwrap();
    /// let status_plantowatch = WatchStatus::from_i32(6).unwrap();
    ///
    /// assert_eq!(status_watching, WatchStatus::Watching);
    /// assert_eq!(status_plantowatch, WatchStatus::PlanToWatch);
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
