//! This module handles adding / updating / removing manga to a user's manga list.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use failure::{Error, ResultExt, SyncFailure};
use MAL;
use MangaInfo;
use minidom::Element;
use super::{ChangeTracker, List, ListEntry, ListType};
use util;

/// Used to perform operations on a user's manga list.
///
/// Note that since the `MangaList` struct stores a reference to a [MAL] instance,
/// the [MAL] instance must live as long as the `MangaList`.
///
/// [MAL]: ../../struct.MAL.html
#[derive(Debug, Copy, Clone)]
pub struct MangaList<'a> {
    /// A reference to the MyAnimeList client used to send requests to the API.
    pub mal: &'a MAL,
}

impl<'a> MangaList<'a> {
    /// Creates a new instance of the `MangaList` struct and stores the provided [MAL] reference
    /// so authorization can be handled automatically.
    ///
    /// [MAL]: ../../struct.MAL.html
    #[inline]
    pub fn new(mal: &'a MAL) -> MangaList<'a> {
        MangaList { mal }
    }
}

impl<'a> List for MangaList<'a> {
    type Entry = MangaEntry;

    #[inline]
    fn list_type() -> ListType {
        ListType::Manga
    }

    #[inline]
    fn mal(&self) -> &MAL {
        self.mal
    }
}

#[derive(Debug, Clone)]
pub struct MangaEntry {
    /// The general series information.
    pub series_info: MangaInfo,
    /// The last time the series was updated.
    pub last_updated_time: DateTime<Utc>,
    chapter: ChangeTracker<u32>,
    volume: ChangeTracker<u32>,
    status: ChangeTracker<ReadStatus>,
    score: ChangeTracker<u8>,
    start_date: ChangeTracker<Option<NaiveDate>>,
    finish_date: ChangeTracker<Option<NaiveDate>>,
    rereading: ChangeTracker<bool>,
    tags: ChangeTracker<Vec<String>>,
}

impl MangaEntry {
    /// Creates a new `MangaEntry` instance with [MangaInfo] obtained from [MAL].
    ///
    /// [MAL]: ../../struct.MAL.html
    /// [MangaInfo]: ../../struct.MangaInfo.html
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

    /// Returns the number of chapters read.
    #[inline]
    pub fn chapter(&self) -> u32 {
        self.chapter.value
    }

    /// Sets the number of chapters read.
    #[inline]
    pub fn set_read_chapters(&mut self, chapter: u32) -> &mut MangaEntry {
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
    pub fn set_read_volumes(&mut self, volume: u32) -> &mut MangaEntry {
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
    pub fn set_status(&mut self, status: ReadStatus) -> &mut MangaEntry {
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
    pub fn set_score(&mut self, score: u8) -> &mut MangaEntry {
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
    pub fn set_start_date(&mut self, date: Option<NaiveDate>) -> &mut MangaEntry {
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
    pub fn set_finish_date(&mut self, date: Option<NaiveDate>) -> &mut MangaEntry {
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
    pub fn set_rereading(&mut self, rereading: bool) -> &mut MangaEntry {
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

impl ListEntry for MangaEntry {
    fn parse(xml_elem: &Element) -> Result<MangaEntry, Error> {
        let get_child =
            |name| util::get_xml_child_text(xml_elem, name).context("failed to parse MAL response");

        let info = MangaInfo {
            id: get_child("series_mangadb_id")?.parse()?,
            title: get_child("series_title")?,
            chapters: get_child("series_chapters")?.parse()?,
            volumes: get_child("series_volumes")?.parse()?,
            start_date: util::parse_str_date(&get_child("series_start")?),
            end_date: util::parse_str_date(&get_child("series_end")?),
            image_url: get_child("series_image")?,
        };

        let entry = MangaEntry {
            series_info: info,
            last_updated_time: Utc.timestamp(get_child("my_last_updated")?.parse()?, 0),
            chapter: get_child("my_read_chapters")?.parse::<u32>()?.into(),
            volume: get_child("my_read_volumes")?.parse::<u32>()?.into(),
            status: ReadStatus::from_i32(get_child("my_status")?.parse()?)?.into(),
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
            tags: super::parse_tags(&get_child("my_tags")?).into(),
        };

        Ok(entry)
    }

    fn generate_xml(&self) -> Result<String, Error> {
        generate_response_xml!(self,
            chapter(num): "chapter" => num.to_string(),
            volume(vol): "volume" => vol.to_string(),
            status(status): "status" => (*status as i32).to_string(),
            score(score): "score" => score.to_string(),
            start_date(date): "date_start" => util::date_to_str(*date),
            finish_date(date): "date_finish" => util::date_to_str(*date),
            rereading(v): "enable_rereading" => (*v as u8).to_string(),
            tags(t): "tags" => super::concat_tags(t)
        )
    }

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

    #[inline]
    fn set_last_updated_time(&mut self) {
        self.last_updated_time = Utc::now();
    }

    #[inline]
    fn id(&self) -> u32 {
        self.series_info.id
    }
}

#[derive(Fail, Debug)]
#[fail(display = "{} does not map to any ReadStatus enum variants", _0)]
pub struct InvalidReadStatus(pub i32);

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
    pub fn from_i32(value: i32) -> Result<ReadStatus, InvalidReadStatus> {
        match value {
            1 => Ok(ReadStatus::Reading),
            2 => Ok(ReadStatus::Completed),
            3 => Ok(ReadStatus::OnHold),
            4 => Ok(ReadStatus::Dropped),
            6 => Ok(ReadStatus::PlanToRead),
            i => Err(InvalidReadStatus(i)),
        }
    }
}

impl Default for ReadStatus {
    #[inline]
    fn default() -> ReadStatus {
        ReadStatus::PlanToRead
    }
}
