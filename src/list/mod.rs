use failure::{Error, SyncFailure};
use MAL;
use minidom::Element;
use request::{self, RequestURL};
use std::fmt::Debug;

macro_rules! generate_response_xml {
    ($struct:ident, $($field:ident($val_name:ident): $xml_name:expr => $xml_val:expr),+) => {{
        let mut entry = Element::bare("entry");

        $(if $struct.$field.changed {
            let $val_name = &$struct.$field.value;

            let mut elem = Element::bare($xml_name);
            elem.append_text_node($xml_val);
            entry.append_child(elem);
        })+

        let mut buffer = Vec::new();
        entry.write_to(&mut buffer).map_err(SyncFailure::new)?;

        Ok(String::from_utf8(buffer)?)
    }};
}

macro_rules! reset_changed_fields {
    ($struct:ident, $($name:ident),+) => ($($struct.$name.changed = false;)+);
}

#[cfg(feature = "anime-list")]
pub mod anime;
#[cfg(feature = "manga-list")]
pub mod manga;

/// Specifies what type of list is being used.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ListType {
    Anime,
    Manga,
}

/// Contains methods that perform common operations on a user's anime / manga list.
pub trait List {
    /// Represents an entry on a user's anime / manga list.
    type Entry: ListEntry;

    /// Requests and parses all entries on a user's anime / manga list.
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
    /// ```
    fn read_entries(&self) -> Result<Vec<Self::Entry>, Error> {
        let req_type = RequestURL::List(&self.mal().username, Self::list_type());
        let resp = request::get_verify(&self.mal().client, req_type)?.text()?;

        let root: Element = resp.parse().map_err(SyncFailure::new)?;
        let mut entries = Vec::new();

        for child in root.children().skip(1) {
            let entry = Self::Entry::parse(child)?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Adds an entry to the user's anime / manga list.
    /// 
    /// If the entry is already on the user's list, nothing will happen.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mal::{MAL, AnimeInfo};
    /// use mal::list::List;
    /// use mal::list::anime::{AnimeEntry, WatchStatus};
    /// 
    /// // Create a new MAL instance
    /// let mal = MAL::new("username", "password");
    /// 
    /// // Search for "Toradora" on MyAnimeList
    /// let mut search_results = mal.search_anime("Toradora").unwrap();
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
    /// mal.anime_list().add(&mut entry).unwrap();
    /// ```
    fn add(&self, entry: &mut Self::Entry) -> Result<(), Error> {
        let body = entry.generate_xml()?;

        request::auth_post_verify(self.mal(),
            RequestURL::Add(entry.id(), Self::list_type()),
            &body)?;

        entry.set_last_updated_time();
        entry.reset_changed_fields();

        Ok(())
    }

    /// Updates an entry on the user's anime / manga list.
    /// 
    /// If the entry is already on the user's list, nothing will happen.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mal::{MAL, AnimeInfo};
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
    /// ```
    fn update(&self, entry: &mut Self::Entry) -> Result<(), Error> {
        let body = entry.generate_xml()?;

        request::auth_post_verify(self.mal(),
            RequestURL::Update(entry.id(), Self::list_type()),
            &body)?;

        entry.set_last_updated_time();
        entry.reset_changed_fields();

        Ok(())
    }

    /// Removes an entry from the user's anime / manga list.
    /// 
    /// If the entry isn't already on the user's list, nothing will happen.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mal::{MAL, AnimeInfo};
    /// use mal::list::List;
    /// use mal::list::anime::{AnimeEntry, WatchStatus};
    /// 
    /// // Create a new MAL instance
    /// let mal = MAL::new("username", "password");
    /// 
    /// // Search for "Toradora" on MyAnimeList
    /// let mut search_results = mal.search_anime("Toradora").unwrap();
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
    fn delete(&self, entry: &Self::Entry) -> Result<(), Error> {
        request::auth_delete_verify(self.mal(),
            RequestURL::Delete(entry.id(), Self::list_type()))?;

        Ok(())
    }

    /// Removes an entry from the user's anime / manga list by its id.
    /// 
    /// If the entry isn't already on the user's list, nothing will happen.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mal::{MAL, AnimeInfo};
    /// use mal::list::List;
    /// use mal::list::anime::{AnimeEntry, WatchStatus};
    /// 
    /// // Create a new MAL instance
    /// let mal = MAL::new("username", "password");
    /// 
    /// // Delete the anime with the id of 4224 (Toradora) from the user's anime list
    /// mal.anime_list().delete_id(4224).unwrap();
    /// ```
    fn delete_id(&self, id: u32) -> Result<(), Error> {
        request::auth_delete_verify(self.mal(), RequestURL::Delete(id, Self::list_type()))?;
        Ok(())
    }

    /// Returns what type of list this is.
    fn list_type() -> ListType;
    /// Returns a reference to the [MAL] client used to send requests to the API.
    /// 
    /// [MAL]: ../struct.MAL.html
    fn mal(&self) -> &MAL;
}

/// Contains required methods to generate and parse list entries from the API.
pub trait ListEntry where Self: Sized {
    /// Used to construct a new version of `Self` from response XML.
    fn parse(xml_elem: &Element) -> Result<Self, Error>;

    /// Used to generate XML to send to the API.
    fn generate_xml(&self) -> Result<String, Error>;
    /// Used to reset the status of any fields that have been modified
    /// since last updating the entry on MyAnimeList.
    fn reset_changed_fields(&mut self);

    /// Used to update the last updated time.
    fn set_last_updated_time(&mut self);

    /// Used to get the MyAnimeList ID of the list entry.
    fn id(&self) -> u32;
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
