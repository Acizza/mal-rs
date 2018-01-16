//! Contains data structures for operating on a user's manga list.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use failure::{Error, SyncFailure};
use MangaInfo;
use minidom::Element;
use request::ListType;
use std::fmt::{self, Display};
use super::{ChangeTracker, EntryValues, ListEntry};
use util;

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

    #[doc(hidden)]
    fn parse(xml_elem: &Element) -> Result<MangaEntry, Error> {
        let get_child = |name| util::get_xml_child_text(xml_elem, name);

        let info = MangaInfo {
            id: get_child("series_mangadb_id")?.parse()?,
            title: get_child("series_title")?,
            synonyms: util::split_into_vec(&get_child("series_synonyms")?, "; "),
            chapters: get_child("series_chapters")?.parse()?,
            volumes: get_child("series_volumes")?.parse()?,
            start_date: util::parse_str_date(&get_child("series_start")?),
            end_date: util::parse_str_date(&get_child("series_end")?),
            image_url: get_child("series_image")?,
        };

        let entry = MangaEntry {
            series_info: info,
            last_updated_time: Utc.timestamp(get_child("my_last_updated")?.parse()?, 0),
            values: MangaValues::parse(xml_elem)?,
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

#[derive(Fail, Debug)]
pub enum MangaValuesError {
    #[fail(display = "{} is not a known read status", _0)] UnknownReadStatus(i32),
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
#[derive(Debug, Clone)]
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
        MangaValues {
            chapter: 0.into(),
            volume: 0.into(),
            status: ReadStatus::default().into(),
            score: 0.into(),
            start_date: None.into(),
            finish_date: None.into(),
            rereading: false.into(),
            tags: Vec::new().into(),
        }
    }

    fn parse(xml_elem: &Element) -> Result<MangaValues, Error> {
        let get_child = |name| util::get_xml_child_text(xml_elem, name);

        let values = MangaValues {
            chapter: get_child("my_read_chapters")?.parse::<u32>()?.into(),
            volume: get_child("my_read_volumes")?.parse::<u32>()?.into(),
            status: {
                let status_num = get_child("my_status")?.parse()?;

                ReadStatus::from_i32(status_num)
                    .ok_or_else(|| MangaValuesError::UnknownReadStatus(status_num))?
                    .into()
            },
            score: get_child("my_score")?.parse::<u8>()?.into(),
            start_date: util::parse_str_date(&get_child("my_start_date")?).into(),
            finish_date: util::parse_str_date(&get_child("my_finish_date")?).into(),
            rereading: {
                // The rereading tag is sometimes blank for no apparent reason..
                get_child("my_rereadingg")?
                    .parse::<u8>()
                    .map(|v| v == 1)
                    .unwrap_or(false)
                    .into()
            },
            tags: util::split_into_vec(&get_child("my_tags")?, ",").into(),
        };

        Ok(values)
    }

    /// Returns the number of chapters read.
    #[inline]
    pub fn chapter(&self) -> u32 {
        self.chapter.value
    }

    /// Sets the number of chapters read.
    #[inline]
    pub fn set_read_chapters(&mut self, chapter: u32) -> &mut MangaValues {
        self.chapter.set(chapter);
        self
    }

    /// Returns the number of volumes read.
    #[inline]
    pub fn volume(&self) -> u32 {
        self.volume.value
    }

    /// Sets the number of volumes read.
    #[inline]
    pub fn set_read_volumes(&mut self, volume: u32) -> &mut MangaValues {
        self.volume.set(volume);
        self
    }

    /// Returns the current reading status of the manga.
    #[inline]
    pub fn status(&self) -> ReadStatus {
        self.status.value
    }

    /// Sets the current read status for the manga.
    #[inline]
    pub fn set_status(&mut self, status: ReadStatus) -> &mut MangaValues {
        self.status.set(status);
        self
    }

    /// Returns the user's score of the manga.
    #[inline]
    pub fn score(&self) -> u8 {
        self.score.value
    }

    /// Sets the user's score for the manga.
    #[inline]
    pub fn set_score(&mut self, score: u8) -> &mut MangaValues {
        self.score.set(score);
        self
    }

    /// Returns the date the manga started being read.
    #[inline]
    pub fn start_date(&self) -> Option<NaiveDate> {
        self.start_date.value
    }

    /// Sets the date the user started reading the manga.
    #[inline]
    pub fn set_start_date(&mut self, date: Option<NaiveDate>) -> &mut MangaValues {
        self.start_date.set(date);
        self
    }

    /// Returns the date the manga finished being read by the user.
    #[inline]
    pub fn finish_date(&self) -> Option<NaiveDate> {
        self.finish_date.value
    }

    /// Sets the date the user finished reading the manga.
    #[inline]
    pub fn set_finish_date(&mut self, date: Option<NaiveDate>) -> &mut MangaValues {
        self.finish_date.set(date);
        self
    }

    /// Returns true if the manga is currently being reread.
    #[inline]
    pub fn rereading(&self) -> bool {
        self.rereading.value
    }

    /// Sets whether or not the user is currently rereading the manga.
    #[inline]
    pub fn set_rereading(&mut self, rereading: bool) -> &mut MangaValues {
        self.rereading.set(rereading);
        self
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
    /// Note that the i32 value of each `ReadStatus` variant is mapped
    /// to the one provided by the MyAnimeList API, so they do not increment naturally.
    ///
    /// # Example
    ///
    /// ```
    /// use mal::list::manga::ReadStatus;
    ///
    /// let status = ReadStatus::from_i32(1).unwrap();
    /// assert_eq!(status, ReadStatus::Reading);
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
