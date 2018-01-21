//! Contains the required data structures to search for manga on MyAnimeList and
//! perform operations on a user's manga list.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use failure::{Error, SyncFailure};
use minidom::Element;
use request::ListType;
use SeriesInfo;
use std::fmt::{self, Display};
use super::{ChangeTracker, EntryValues, ListEntry, UserInfo};
use util::{self, parse_xml_child};

#[derive(Fail, Debug)]
pub enum MangaError {
    #[fail(display = "\"{}\" does not map to a known publishing status", _0)]
    UnknownPublishStatus(String),

    #[fail(display = "{} is not a known read status", _0)]
    UnknownReadStatus(i32),

    #[fail(display = "\"{}\" does not map to a known series type", _0)]
    UnknownSeriesType(String),
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
    /// The current publishing status of the series.
    pub publishing_status: PublishingStatus,
    /// The date the series started airing.
    pub start_date: Option<NaiveDate>,
    /// The date the series finished airing.
    pub end_date: Option<NaiveDate>,
    /// The URL to the cover image of the series.
    pub image_url: String,
}

impl SeriesInfo for MangaInfo {
    #[doc(hidden)]
    fn parse_search_result(xml: &Element) -> Result<MangaInfo, Error> {
        let entry = MangaInfo {
            id: parse_xml_child(xml, "id")?,
            title: parse_xml_child(xml, "title")?,
            synonyms: util::split_into_vec(&parse_xml_child::<String>(xml, "synonyms")?, "; "),
            series_type: {
                let s_type = parse_xml_child(xml, "type")?;

                MangaType::from_str(&s_type).ok_or_else(|| MangaError::UnknownSeriesType(s_type))?
            },
            chapters: parse_xml_child(xml, "chapters")?,
            volumes: parse_xml_child(xml, "volumes")?,
            publishing_status: {
                let status = parse_xml_child(xml, "status")?;

                PublishingStatus::from_str(&status)
                    .ok_or_else(|| MangaError::UnknownPublishStatus(status))?
            },
            start_date: util::parse_str_date(&parse_xml_child::<String>(xml, "start_date")?),
            end_date: util::parse_str_date(&parse_xml_child::<String>(xml, "end_date")?),
            image_url: parse_xml_child(xml, "image")?,
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
    /// Attempts to convert an i32 to a `MangaType`.
    ///
    /// # Example
    ///
    /// ```
    /// use mal::list::manga::MangaType;
    ///
    /// let type_manga = MangaType::from_i32(1).unwrap();
    /// let type_manhua = MangaType::from_i32(6).unwrap();
    ///
    /// assert_eq!(type_manga, MangaType::Manga);
    /// assert_eq!(type_manhua, MangaType::Manhua);
    /// ```
    #[inline]
    pub fn from_i32(value: i32) -> Option<MangaType> {
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

    fn from_str<S: AsRef<str>>(input: S) -> Option<MangaType> {
        let lowered = input.as_ref().to_ascii_lowercase();

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

/// Represents the current airing status of a series.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PublishingStatus {
    Publishing = 1,
    Finished,
    NotYetPublished,
}

impl PublishingStatus {
    /// Attempts to convert an i32 to a `PublishingStatus`.
    ///
    /// # Example
    ///
    /// ```
    /// use mal::list::manga::PublishingStatus;
    ///
    /// let status_publishing = PublishingStatus::from_i32(1).unwrap();
    /// let status_notpublished = PublishingStatus::from_i32(3).unwrap();
    ///
    /// assert_eq!(status_publishing, PublishingStatus::Publishing);
    /// assert_eq!(status_notpublished, PublishingStatus::NotYetPublished);
    /// ```
    #[inline]
    pub fn from_i32(value: i32) -> Option<PublishingStatus> {
        match value {
            1 => Some(PublishingStatus::Publishing),
            2 => Some(PublishingStatus::Finished),
            3 => Some(PublishingStatus::NotYetPublished),
            _ => None,
        }
    }

    fn from_str<S: AsRef<str>>(input: S) -> Option<PublishingStatus> {
        let lowered = input.as_ref().to_ascii_lowercase();

        match lowered.as_str() {
            "publishing" => Some(PublishingStatus::Publishing),
            "finished" => Some(PublishingStatus::Finished),
            "not yet published" => Some(PublishingStatus::NotYetPublished),
            _ => None,
        }
    }
}

/// Contains information about a manga series on a user's list.
#[derive(Debug, Clone)]
pub struct MangaEntry {
    /// The general series information.
    pub series_info: MangaInfo,
    /// The last time the series was updated.
    pub last_updated_time: DateTime<Utc>,
    /// Contains values that can be set / updated on a user's list.
    pub values: MangaValues,
}

impl MangaEntry {
    /// Creates a new `MangaEntry` instance with [`MangaInfo`] obtained from [`MAL`].
    ///
    /// [`MAL`]: ../../struct.MAL.html
    /// [`MangaInfo`]: ../../struct.MangaInfo.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use mal::MAL;
    /// use mal::list::manga::MangaEntry;
    ///
    /// // Create a new MAL instance
    /// let mal = MAL::new("username", "password");
    ///
    /// // Search for Bleach on MAL
    /// let mut results = mal.search_manga("Bleach").unwrap();
    ///
    /// // Select the first result
    /// let bleach_info = results.swap_remove(0);
    ///
    /// // Create a new MangaEntry that represents Bleach with default values
    /// let entry = MangaEntry::new(bleach_info);
    /// ```
    #[inline]
    pub fn new(info: MangaInfo) -> MangaEntry {
        MangaEntry {
            series_info: info,
            last_updated_time: Utc::now(),
            values: MangaValues::new(),
        }
    }
}

impl ListEntry for MangaEntry {
    type Values = MangaValues;
    type UserInfo = MangaUserInfo;

    #[doc(hidden)]
    fn parse(xml: &Element) -> Result<MangaEntry, Error> {
        let info = MangaInfo {
            id: parse_xml_child(xml, "series_mangadb_id")?,
            title: parse_xml_child(xml, "series_title")?,
            synonyms: {
                util::split_into_vec(&parse_xml_child::<String>(xml, "series_synonyms")?, "; ")
            },
            series_type: {
                let s_type = parse_xml_child(xml, "series_type")?;

                MangaType::from_i32(s_type)
                    .ok_or_else(|| MangaError::UnknownSeriesType(s_type.to_string()))?
            },
            chapters: parse_xml_child(xml, "series_chapters")?,
            volumes: parse_xml_child(xml, "series_volumes")?,
            publishing_status: {
                let status = parse_xml_child(xml, "series_status")?;

                PublishingStatus::from_i32(status)
                    .ok_or_else(|| MangaError::UnknownPublishStatus(status.to_string()))?
            },
            start_date: util::parse_str_date(&parse_xml_child::<String>(xml, "series_start")?),
            end_date: util::parse_str_date(&parse_xml_child::<String>(xml, "series_end")?),
            image_url: parse_xml_child(xml, "series_image")?,
        };

        let entry = MangaEntry {
            series_info: info,
            last_updated_time: Utc.timestamp(parse_xml_child(xml, "my_last_updated")?, 0),
            values: MangaValues::parse(xml)?,
        };

        Ok(entry)
    }

    #[doc(hidden)]
    #[inline]
    fn values_mut(&mut self) -> &mut MangaValues {
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
        ListType::Manga
    }
}

/// Contains values that can set / updated on a user's list.
///
/// # Examples
///
/// ```
/// use mal::list::manga::{MangaValues, ReadStatus};
///
/// let mut values = MangaValues::new();
///
/// values.set_read_chapters(50)
///       .set_read_volumes(2)
///       .set_status(ReadStatus::Reading)
///       .set_score(7);
///
/// assert_eq!(values.chapter(), 50);
/// assert_eq!(values.volume(), 2);
/// assert_eq!(values.status(), ReadStatus::Reading);
/// assert_eq!(values.score(), 7);
/// ```
#[derive(Debug, Default, Clone)]
pub struct MangaValues {
    chapter: ChangeTracker<u32>,
    volume: ChangeTracker<u32>,
    status: ChangeTracker<ReadStatus>,
    score: ChangeTracker<u8>,
    start_date: ChangeTracker<Option<NaiveDate>>,
    finish_date: ChangeTracker<Option<NaiveDate>>,
    rereading: ChangeTracker<bool>,
    tags: ChangeTracker<Vec<String>>,
}

impl MangaValues {
    /// Creates a new `MangaValues` instance with default values.
    #[inline]
    pub fn new() -> MangaValues {
        MangaValues::default()
    }

    fn parse(xml: &Element) -> Result<MangaValues, Error> {
        let values = MangaValues {
            chapter: parse_xml_child::<u32>(xml, "my_read_chapters")?.into(),
            volume: parse_xml_child::<u32>(xml, "my_read_volumes")?.into(),
            status: {
                let status_num = parse_xml_child(xml, "my_status")?;

                ReadStatus::from_i32(status_num)
                    .ok_or_else(|| MangaError::UnknownReadStatus(status_num))?
                    .into()
            },
            score: parse_xml_child::<u8>(xml, "my_score")?.into(),
            start_date: {
                util::parse_str_date(&parse_xml_child::<String>(xml, "my_start_date")?).into()
            },
            finish_date: {
                util::parse_str_date(&parse_xml_child::<String>(xml, "my_finish_date")?).into()
            },
            rereading: {
                // The rereading tag is sometimes blank for no apparent reason..
                parse_xml_child::<u8>(xml, "my_rereadingg")
                    .map(|v| v == 1)
                    .unwrap_or(false)
                    .into()
            },
            tags: util::split_into_vec(&parse_xml_child::<String>(xml, "my_tags")?, ",").into(),
        };

        Ok(values)
    }

    /// Returns the tags the user has set for the manga.
    #[inline]
    pub fn tags(&self) -> &Vec<String> {
        &self.tags.value
    }

    /// Returns a mutable reference to the tags the user has set for the manga.
    #[inline]
    pub fn tags_mut(&mut self) -> &mut Vec<String> {
        // If a mutable reference is being requested, then it's safe to assume the values
        // are going to be changed
        self.tags.changed = true;
        &mut self.tags.value
    }
}

impl_tracker_getset!(MangaValues,
    [chapter, set_read_chapters, "number of read chapters"]: u32,
    [volume, set_read_volumes, "number of read volumes"]: u32,
    [status, set_status, "current reading status of the series"]: ReadStatus,
    [score, set_score, "user's rating of the series"]: u8,
    [start_date, set_start_date, "date the user started reading the series"]: Option<NaiveDate>,
    [finish_date, set_finish_date, "date the user finished reading the series"]: Option<NaiveDate>,
    [rereading, set_rereading, "current re-reading status of the series"]: bool,
);

impl EntryValues for MangaValues {
    #[doc(hidden)]
    fn generate_xml(&self) -> Result<String, Error> {
        generate_response_xml!(self,
            chapter(num): "chapter" => num.to_string(),
            volume(vol): "volume" => vol.to_string(),
            status(status): "status" => (*status as i32).to_string(),
            score(score): "score" => score.to_string(),
            start_date(date): "date_start" => util::date_to_str(*date),
            finish_date(date): "date_finish" => util::date_to_str(*date),
            rereading(v): "enable_rereading" => (*v as u8).to_string(),
            tags(t): "tags" => util::concat_by_delimeter(t, ',')
        )
    }

    #[doc(hidden)]
    #[inline]
    fn reset_changed_fields(&mut self) {
        reset_changed_fields!(
            self,
            chapter,
            volume,
            status,
            score,
            start_date,
            finish_date,
            rereading,
            tags
        );
    }
}

/// Contains list statistics and user information.
#[derive(Debug, Clone)]
pub struct MangaUserInfo {
    /// The user's ID.
    pub user_id: u32,
    /// The number of manga being read.
    pub reading: u32,
    /// The number of manga completed.
    pub completed: u32,
    /// The number of manga on hold.
    pub on_hold: u32,
    /// The number of manga dropped.
    pub dropped: u32,
    /// The number of manga that are planning to be read.
    pub plan_to_read: u32,
    /// The total days spent reading all of the manga on the user's list.
    pub days_spent_watching: f32,
}

impl UserInfo for MangaUserInfo {
    #[doc(hidden)]
    fn parse(xml: &Element) -> Result<MangaUserInfo, Error> {
        let info = MangaUserInfo {
            user_id: parse_xml_child(xml, "user_id")?,
            reading: parse_xml_child(xml, "user_reading")?,
            completed: parse_xml_child(xml, "user_completed")?,
            on_hold: parse_xml_child(xml, "user_onhold")?,
            dropped: parse_xml_child(xml, "user_dropped")?,
            plan_to_read: parse_xml_child(xml, "user_plantoread")?,
            days_spent_watching: parse_xml_child(xml, "user_days_spent_watching")?,
        };

        Ok(info)
    }
}

/// Represents the read status of a manga on the user's list.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ReadStatus {
    Reading = 1,
    Completed,
    OnHold,
    Dropped,
    PlanToRead = 6,
}

impl ReadStatus {
    /// Attempts to convert an i32 to a `ReadStatus`.
    ///
    /// # Example
    ///
    /// ```
    /// use mal::list::manga::ReadStatus;
    ///
    /// let status_reading = ReadStatus::from_i32(1).unwrap();
    /// let status_plantoread = ReadStatus::from_i32(6).unwrap();
    ///
    /// assert_eq!(status_reading, ReadStatus::Reading);
    /// assert_eq!(status_plantoread, ReadStatus::PlanToRead);
    /// ```
    #[inline]
    pub fn from_i32(value: i32) -> Option<ReadStatus> {
        match value {
            1 => Some(ReadStatus::Reading),
            2 => Some(ReadStatus::Completed),
            3 => Some(ReadStatus::OnHold),
            4 => Some(ReadStatus::Dropped),
            6 => Some(ReadStatus::PlanToRead),
            _ => None,
        }
    }
}

impl Default for ReadStatus {
    #[inline]
    fn default() -> ReadStatus {
        ReadStatus::PlanToRead
    }
}

impl Display for ReadStatus {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ReadStatus::Reading => write!(f, "reading"),
            ReadStatus::Completed => write!(f, "completed"),
            ReadStatus::OnHold => write!(f, "on hold"),
            ReadStatus::Dropped => write!(f, "dropped"),
            ReadStatus::PlanToRead => write!(f, "plan to read"),
        }
    }
}
