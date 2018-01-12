//! This module handles adding / updating / removing anime to a user's list.

use chrono::NaiveDate;
use failure::{Error, ResultExt, SyncFailure};
use MAL;
use minidom::Element;
use request;
use RequestURL;
use SeriesInfo;
use std::fmt::Debug;
use super::List;
use util;

/// Used to perform operations on a user's anime list.
/// 
/// Note that since the `AnimeList` struct stores a reference to a [MAL] instance,
/// the [MAL] instance must live as long as the `AnimeList`.
/// 
/// [MAL]: ../struct.MAL.html
#[derive(Debug, Copy, Clone)]
pub struct AnimeList<'a> {
    /// A reference to the MyAnimeList client used to add and update anime on a user's list.
    pub mal: &'a MAL,
}

impl<'a> AnimeList<'a> {
    /// Creates a new instance of the `AnimeList` struct and stores the provided [MAL] reference
    /// so authorization can be handled automatically.
    /// 
    /// [MAL]: ../struct.MAL.html
    #[inline]
    pub fn new(mal: &'a MAL) -> AnimeList<'a> {
        AnimeList { mal }
    }
}

impl<'a> List for AnimeList<'a> {
    type Entry = AnimeEntry;

    /// Requests and parses all entries on the user's anime list.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mal::MAL;
    /// use mal::list::List;
    /// 
    /// // Create a new MAL instance
    /// let mal = MAL::new("username", "password");
    /// 
    /// // Read all list entries from the user's list
    /// let entries = mal.anime_list().read_entries().unwrap();
    /// 
    /// assert!(entries.len() > 0);
    /// ```
    fn read_entries(&self) -> Result<Vec<AnimeEntry>, Error> {
        let resp = request::get_verify(&self.mal.client, RequestURL::AnimeList(&self.mal.username))?.text()?;
        let root: Element = resp.parse().map_err(SyncFailure::new)?;

        let mut entries = Vec::new();

        for child in root.children().skip(1) {
            let get_child = |name| {
                util::get_xml_child_text(child, name)
                    .context("failed to parse MAL response")
            };

            let info = SeriesInfo {
                id: get_child("series_animedb_id")?.parse()?,
                title: get_child("series_title")?,
                episodes: get_child("series_episodes")?.parse()?,
                start_date: util::parse_str_date(&get_child("series_start")?),
                end_date: util::parse_str_date(&get_child("series_end")?),
            };

            let entry = AnimeEntry {
                series_info: info,
                watched_episodes: get_child("my_watched_episodes")?.parse::<u32>()?.into(),
                start_date: util::parse_str_date(&get_child("my_start_date")?).into(),
                finish_date: util::parse_str_date(&get_child("my_finish_date")?).into(),
                status: WatchStatus::from_i32(get_child("my_status")?.parse()?)?.into(),
                score: get_child("my_score")?.parse::<u8>()?.into(),
                rewatching: {
                    // The rewatching tag is sometimes blank for no apparent reason..
                    get_child("my_rewatching")?
                        .parse::<u8>()
                        .map(|v| v == 1)
                        .unwrap_or(false)
                        .into()
                },
                tags: parse_tags(&get_child("my_tags")?).into(),
            };

            entries.push(entry);
        }

        Ok(entries)
    }

    /// Adds an anime to the user's list.
    /// 
    /// If the anime is already on the user's list, nothing will happen.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mal::{MAL, SeriesInfo};
    /// use mal::list::List;
    /// use mal::list::anime::{AnimeEntry, WatchStatus};
    /// 
    /// // Create a new MAL instance
    /// let mal = MAL::new("username", "password");
    /// 
    /// // Search for "Toradora" on MyAnimeList
    /// let mut search_results = mal.search("Toradora").unwrap();
    /// 
    /// // Use the first result's info
    /// let toradora_info = search_results.swap_remove(0);
    /// 
    /// // Create a new anime list entry with Toradora's info
    /// let mut entry = AnimeEntry::new(toradora_info);
    /// 
    /// // Set the entry's watched episodes to 5 and status to watching
    /// entry.set_watched_episodes(5).set_status(WatchStatus::Watching);
    /// 
    /// // Add the entry to the user's anime list
    /// mal.anime_list().add(&entry).unwrap();
    /// ```
    #[inline]
    fn add(&self, entry: &AnimeEntry) -> Result<(), Error> {
        let body = entry.generate_xml()?;

        request::auth_post_verify(self.mal,
            RequestURL::AddAnime(entry.series_info.id),
            &body)?;

        Ok(())
    }

    /// Updates the specified anime on the user's list.
    /// 
    /// If the anime is already on the user's list, nothing will happen.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mal::{MAL, SeriesInfo};
    /// use mal::list::List;
    /// use mal::list::anime::{AnimeEntry, WatchStatus};
    /// 
    /// // Create a new MAL instance
    /// let mal = MAL::new("username", "password");
    /// 
    /// // Get a handle to the user's anime list
    /// let anime_list = mal.anime_list();
    /// 
    /// // Get and parse all of the list entries
    /// let entries = anime_list.read_entries().unwrap();
    /// 
    /// // Find Toradora in the list entries
    /// let mut toradora_entry = entries.into_iter().find(|e| e.series_info.id == 4224).unwrap();
    /// 
    /// // Set new values for the list entry
    /// // In this case, the episode count will be updated to 25, the score will be set to 10, and the status will be set to completed
    /// toradora_entry.set_watched_episodes(25)
    ///               .set_score(10)
    ///               .set_status(WatchStatus::Completed);
    /// 
    /// // Update the anime on the user's list
    /// anime_list.update(&mut toradora_entry).unwrap();
    /// 
    /// assert_eq!(toradora_entry.watched_episodes(), 25);
    /// assert_eq!(toradora_entry.status(), WatchStatus::Completed);
    /// assert_eq!(toradora_entry.score(), 10);
    /// ```
    #[inline]
    fn update(&self, entry: &mut AnimeEntry) -> Result<(), Error> {
        let body = entry.generate_xml()?;
        
        request::auth_post_verify(self.mal,
            RequestURL::UpdateAnime(entry.series_info.id),
            &body)?;

        entry.reset_changed_status();
        Ok(())
    }

    /// Removes an anime from the user's list.
    /// 
    /// If the anime isn't already on the user's list, nothing will happen.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mal::{MAL, SeriesInfo};
    /// use mal::list::List;
    /// use mal::list::anime::{AnimeEntry, WatchStatus};
    /// 
    /// // Create a new MAL instance
    /// let mal = MAL::new("username", "password");
    /// 
    /// // Search for "Toradora" on MyAnimeList
    /// let mut search_results = mal.search("Toradora").unwrap();
    /// 
    /// // Use the first result's info
    /// let toradora_info = search_results.swap_remove(0);
    /// 
    /// // Get a handle to the user's anime list
    /// let anime_list = mal.anime_list();
    /// 
    /// // Get and parse all of the list entries
    /// let entries = anime_list.read_entries().unwrap();
    /// 
    /// // Find Toradora in the list entries
    /// let toradora_entry = entries.into_iter().find(|e| e.series_info.id == 4224).unwrap();
    /// 
    /// // Delete Toradora from the user's anime list
    /// anime_list.delete(&toradora_entry).unwrap();
    /// ```
    #[inline]
    fn delete(&self, entry: &AnimeEntry) -> Result<(), Error> {
        request::auth_delete_verify(self.mal,
            RequestURL::DeleteAnime(entry.series_info.id))?;

        Ok(())
    }

    /// Removes an anime from the user's list by its id on MyAnimeList.
    /// 
    /// If the anime isn't already on the user's list, nothing will happen.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mal::{MAL, SeriesInfo};
    /// use mal::list::List;
    /// use mal::list::anime::{AnimeEntry, WatchStatus};
    /// 
    /// // Create a new MAL instance
    /// let mal = MAL::new("username", "password");
    /// 
    /// // Delete the anime with the id of 4224 (Toradora) from the user's anime list
    /// mal.anime_list().delete_id(4224).unwrap();
    /// ```
    #[inline]
    fn delete_id(&self, id: u32) -> Result<(), Error> {
        request::auth_delete_verify(self.mal, RequestURL::DeleteAnime(id))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ChangeTracker<T: Debug + Clone> {
    value: T,
    changed: bool,
}

impl<T: Debug + Clone> ChangeTracker<T> {
    fn new(value: T) -> ChangeTracker<T> {
        ChangeTracker {
            value,
            changed: false,
        }
    }

    fn set(&mut self, value: T) {
        self.value = value;
        self.changed = true;
    }
}

impl<T: Debug + Clone> From<T> for ChangeTracker<T> {
    fn from(value: T) -> Self {
        ChangeTracker::new(value)
    }
}

/// Represents information about an anime series on a user's list.
#[derive(Debug, Clone)]
pub struct AnimeEntry {
    /// The general series information.
    pub series_info: SeriesInfo,
    watched_episodes: ChangeTracker<u32>,
    start_date: ChangeTracker<Option<NaiveDate>>,
    finish_date: ChangeTracker<Option<NaiveDate>>,
    status: ChangeTracker<WatchStatus>,
    score: ChangeTracker<u8>,
    rewatching: ChangeTracker<bool>,
    tags: ChangeTracker<Vec<String>>,
}

impl AnimeEntry {
    /// Creates a new `AnimeEntry` instance with [SeriesInfo] obtained from [MAL].
    /// 
    /// [MAL]: ../struct.MAL.html
    /// [SeriesInfo]: ../struct.SeriesInfo.html
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
    /// let mut results = mal.search("Toradora").unwrap();
    /// 
    /// // Select the first result
    /// let toradora_info = results.swap_remove(0);
    /// 
    /// // Create a new AnimeEntry that represents Toradora with default values
    /// let entry = AnimeEntry::new(toradora_info);
    /// ```
    #[inline]
    pub fn new(info: SeriesInfo) -> AnimeEntry {
        AnimeEntry {
            series_info: info,
            watched_episodes: 0.into(),
            start_date: None.into(),
            finish_date: None.into(),
            status: WatchStatus::default().into(),
            score: 0.into(),
            rewatching: false.into(),
            tags: Vec::new().into(),
        }
    }

    fn generate_xml(&self) -> Result<String, Error> {
        macro_rules! gen_xml {
            ($entry:ident, $xml_elem:ident, $($field:ident($val_name:ident): $xml_name:expr => $xml_val:expr),+) => {
                $(if $entry.$field.changed {
                    let $val_name = &$entry.$field.value;

                    let mut elem = Element::bare($xml_name);
                    elem.append_text_node($xml_val);
                    $xml_elem.append_child(elem);
                })+
            };
        }

        let mut entry = Element::bare("entry");

        gen_xml!(self, entry,
            watched_episodes(num): "episode" => num.to_string(),
            status(status): "status" => (*status as i32).to_string(),
            start_date(date): "date_start" => util::date_to_str(*date),
            finish_date(date): "date_finish" => util::date_to_str(*date),
            score(score): "score" => score.to_string(),
            rewatching(v): "enable_rewatching" => (*v as u8).to_string(),
            tags(t): "tags" => concat_tags(t)
        );

        let mut buffer = Vec::new();
        entry.write_to(&mut buffer).map_err(SyncFailure::new)?;

        Ok(String::from_utf8(buffer)?)
    }

    fn reset_changed_status(&mut self) {
        macro_rules! reset {
            ($($name:ident),+) => ($(self.$name.changed = false;)+);
        }

        reset! {
            watched_episodes,
            start_date,
            finish_date,
            status,
            score,
            rewatching,
            tags
        }
    }

    /// Returns the number of episodes watched.
    #[inline]
    pub fn watched_episodes(&self) -> u32 {
        self.watched_episodes.value
    }

    /// Sets the watched episode count.
    #[inline]
    pub fn set_watched_episodes(&mut self, watched: u32) -> &mut AnimeEntry {
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
    pub fn set_start_date(&mut self, date: Option<NaiveDate>) -> &mut AnimeEntry {
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
    pub fn set_finish_date(&mut self, date: Option<NaiveDate>) -> &mut AnimeEntry {
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
    pub fn set_status(&mut self, status: WatchStatus) -> &mut AnimeEntry {
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
    pub fn set_score(&mut self, score: u8) -> &mut AnimeEntry {
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
    pub fn set_rewatching(&mut self, rewatching: bool) -> &mut AnimeEntry {
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

impl PartialEq for AnimeEntry {
    #[inline]
    fn eq(&self, other: &AnimeEntry) -> bool {
        self.series_info == other.series_info
    }
}

fn parse_tags(tag_str: &str) -> Vec<String> {
    tag_str.split(',').map(|s| s.to_string()).collect()
}

fn concat_tags(tags: &[String]) -> String {
    tags.iter().map(|tag| format!("{},", tag)).collect()
}

#[derive(Fail, Debug)]
#[fail(display = "{} does not map to any WatchStatus enum variants", _0)]
pub struct InvalidWatchStatus(pub i32);

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
    pub fn from_i32(value: i32) -> Result<WatchStatus, InvalidWatchStatus> {
        match value {
            1 => Ok(WatchStatus::Watching),
            2 => Ok(WatchStatus::Completed),
            3 => Ok(WatchStatus::OnHold),
            4 => Ok(WatchStatus::Dropped),
            6 => Ok(WatchStatus::PlanToWatch),
            i => Err(InvalidWatchStatus(i)),
        }
    }
}

impl Default for WatchStatus {
    #[inline]
    fn default() -> Self {
        WatchStatus::PlanToWatch
    }
}