use failure::{Error, SyncFailure};
use MAL;
use minidom::Element;
use request::Request;
use std::fmt::{self, Debug, Display};

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

impl Display for ListType {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ListType::Anime => write!(f, "anime"),
            ListType::Manga => write!(f, "manga"),
        }
    }
}

/// Contains methods that perform common operations on a user's list.
pub trait List {
    /// Represents an entry on a user's list.
    type Entry: ListEntry<Self::EntryValues>;

    // This only exists here because putting it in the ListEntry trait (where it makes more sense)
    // causes an ambiguous associated type error. Using this type as a type parameter
    // for the Entry type avoids the issue when placed here
    /// Represents values that can be modified on a user's list.
    type EntryValues: EntryValues;

    /// Requests and parses all entries on a user's list.
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
        let resp = Request::List(&self.mal().username, Self::list_type())
            .send(self.mal())?
            .text()?;

        let root: Element = resp.parse().map_err(SyncFailure::new)?;
        let mut entries = Vec::new();

        for child in root.children().skip(1) {
            let entry = Self::Entry::parse(child)?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Adds an entry to a user's list.
    /// 
    /// If the entry is already on a user's list, nothing will happen.
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
    /// entry.values
    ///      .set_watched_episodes(5)
    ///      .set_status(WatchStatus::Watching);
    /// 
    /// // Add the entry to the user's anime list
    /// mal.anime_list().add(&mut entry).unwrap();
    /// ```
    #[inline]
    fn add(&self, entry: &mut Self::Entry) -> Result<(), Error> {
        self.add_id(entry.id(), entry.values_mut())?;
        entry.set_last_updated_time();
        Ok(())
    }

    /// Adds an entry to a user's list by id.
    /// 
    /// If the entry is already on a user's list, nothing will happen.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mal::{MAL, AnimeInfo};
    /// use mal::list::List;
    /// use mal::list::anime::{AnimeEntry, AnimeValues, WatchStatus};
    /// 
    /// // Create a new MAL instance
    /// let mal = MAL::new("username", "password");
    /// 
    /// // Create new entry values
    /// let mut values = AnimeValues::new();
    /// 
    /// // Set the number of watched episodes to 5 and the status to watching
    /// values.set_watched_episodes(5)
    ///       .set_status(WatchStatus::Watching);
    /// 
    /// // Add an entry with an id of 4224 (Toradora) to the user's anime list
    /// mal.anime_list().add_id(4224, &mut values).unwrap();
    /// ```
    fn add_id(&self, id: u32, values: &mut Self::EntryValues) -> Result<(), Error> {
        let body = values.generate_xml()?;

        Request::Add(id, Self::list_type(), &body)
            .send(self.mal())?;

        values.reset_changed_fields();
        Ok(())
    }

    /// Updates an entry on a user's list.
    /// 
    /// If the entry is already on a user's list, nothing will happen.
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
    /// let mut toradora = entries.into_iter().find(|e| e.series_info.id == 4224).unwrap();
    /// 
    /// // Set new values for the list entry
    /// // In this case, the episode count will be updated to 25, the score will be set to 10, and the status will be set to completed
    /// toradora.values
    ///         .set_watched_episodes(25)
    ///         .set_score(10)
    ///         .set_status(WatchStatus::Completed);
    /// 
    /// // Update the anime on the user's list
    /// anime_list.update(&mut toradora).unwrap();
    /// ```
    #[inline]
    fn update(&self, entry: &mut Self::Entry) -> Result<(), Error> {
        self.update_id(entry.id(), entry.values_mut())?;
        entry.set_last_updated_time();
        Ok(())
    }

    /// Updates an entry on a user's list by id.
    /// 
    /// If the entry is already on the user's list, nothing will happen.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mal::{MAL, AnimeInfo};
    /// use mal::list::List;
    /// use mal::list::anime::{AnimeEntry, AnimeValues, WatchStatus};
    /// 
    /// // Create a new MAL instance
    /// let mal = MAL::new("username", "password");
    /// 
    /// // Create new entry values
    /// let mut values = AnimeValues::new();
    /// 
    /// // Set the number of watched episodes to 25, score to 10, and status to completed
    /// values.set_watched_episodes(25)
    ///       .set_score(10)
    ///       .set_status(WatchStatus::Completed);
    /// 
    /// // Update the entry with an id of 4224 (Toradora) on the user's anime list
    /// mal.anime_list().update_id(4224, &mut values).unwrap();
    /// ```
    fn update_id(&self, id: u32, values: &mut Self::EntryValues) -> Result<(), Error> {
        let body = values.generate_xml()?;

        Request::Update(id, Self::list_type(), &body)
            .send(self.mal())?;

        values.reset_changed_fields();
        Ok(())
    }

    /// Removes an entry from a user's list.
    /// 
    /// If the entry isn't already on a user's list, nothing will happen.
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
    #[inline]
    fn delete(&self, entry: &Self::Entry) -> Result<(), Error> {
        self.delete_id(entry.id())
    }

    /// Removes an entry from a user's list by its id.
    /// 
    /// If the entry isn't already on a user's list, nothing will happen.
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
    #[inline]
    fn delete_id(&self, id: u32) -> Result<(), Error> {
        Request::Delete(id, Self::list_type())
            .send(self.mal())?;
        
        Ok(())
    }

    /// Indicates what type of list this is.
    fn list_type() -> ListType;

    /// Returns a reference to the [MAL] client used to send requests to the API.
    /// 
    /// [MAL]: ../struct.MAL.html
    fn mal(&self) -> &MAL;
}

/// Represents an entry on a user's list.
pub trait ListEntry<V: EntryValues> where Self: Sized {
    #[doc(hidden)]
    fn parse(xml_elem: &Element) -> Result<Self, Error>;

    #[doc(hidden)]
    fn values_mut(&mut self) -> &mut V;

    #[doc(hidden)]
    fn set_last_updated_time(&mut self);

    #[doc(hidden)]
    fn id(&self) -> u32;
}

/// Represents values on a user's list that can be set.
pub trait EntryValues {
    #[doc(hidden)]
    fn generate_xml(&self) -> Result<String, Error>;

    #[doc(hidden)]
    fn reset_changed_fields(&mut self);
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
