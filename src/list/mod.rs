//! This module provides generic functionality for adding, updating, deleting, and reading entries
//! from a user's anime / manga list.
//! 
//! All functions that perform operations on a user's list are located in the [`List`] struct,
//! and list-specific data structures are located in the [`anime`] and [`manga`] modules.
//! 
//! [`List`]: ./struct.List.html
//! [`anime`]: ./anime/index.html
//! [`manga`]: ./manga/index.html
//! 
//! # Examples
//! 
//! Adding an anime to a user's list:
//! 
//! ```no_run
//! use mal::MAL;
//! use mal::list::anime::{AnimeEntry, WatchStatus};
//! 
//! // Create a new MAL instance
//! let mal = MAL::new("username", "password");
//! 
//! // Search for "Toradora" on MyAnimeList
//! let mut search_results = mal.search_anime("Toradora").unwrap();
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
//!      .set_status(WatchStatus::Watching);
//! 
//! // Add the entry to the user's anime list
//! mal.anime_list().add(&mut entry).unwrap();
//! ```
//! 
//! Updating a manga on a user's list by its ID:
//! 
//! ```no_run
//! use mal::MAL;
//! use mal::list::manga::{MangaValues, ReadStatus};
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
//!       .set_status(ReadStatus::Completed);
//! 
//! // Update the entry with an id of 2 (Berserk) on the user's manga list with the specified values
//! mal.manga_list().update_id(2, &mut values).unwrap();
//! ```
//! 
//! Retrieving an anime off of a user's list and updating it:
//! 
//! ```no_run
//! use mal::MAL;
//! use mal::list::anime::WatchStatus;
//! 
//! // Create a new MAL instance
//! let mal = MAL::new("username", "password");
//! 
//! // Read the user's anime list
//! let list = mal.anime_list().read().unwrap();
//! 
//! // Find the first series on the user's list that's being watched
//! let mut entry = list.entries.into_iter().find(|e| {
//!     e.values.status() == WatchStatus::Watching
//! }).unwrap();
//! 
//! // Set the entrie's watched episodes to its total episodes, its score to 10, and status to completed
//! entry.values
//!      .set_watched_episodes(entry.series_info.episodes)
//!      .set_score(10)
//!      .set_status(WatchStatus::Completed);
//! 
//! // Update the entry on the user's anime list with the new values
//! mal.anime_list().update(&mut entry).unwrap();
//! ```

use failure::{Error, SyncFailure};
use {MAL, MALError};
use minidom::Element;
use request::{ListType, Request};
use std::fmt::Debug;
use std::marker::PhantomData;

// Generates getter and setter methods for struct fields with a ChangeTracker type.
macro_rules! impl_tracker_getset {
    ($name:ident, $([$field:ident, $setter:ident, $verb:expr]: $field_type:ty,)+) => {
        impl $name {
            $(
            #[doc = "Returns the "]
            #[doc = $verb]
            #[doc = "."]
            #[inline]
            pub fn $field(&self) -> $field_type {
                self.$field.value
            }

            #[doc = "Sets the "]
            #[doc = $verb]
            #[doc = "."]
            #[inline]
            pub fn $setter(&mut self, $field: $field_type) -> &mut $name {
                self.$field.set($field);
                self
            }
            )+
        }
    };
}

// Generates enums that can be parsed from search results and a user's list
macro_rules! gen_list_field_enum {
    ($name:ident, $([$field_doc:expr] $field:ident = [$field_index:expr, $field_str:expr],)+) => {
        #[derive(Debug, Copy, Clone, PartialEq)]
        pub enum $name {
            $(
            #[doc = $field_doc]
            $field = $field_index,
            )+
        }

        impl $name {
            #[inline]
            pub fn from_i32(value: i32) -> Option<$name> {
                match value {
                    $($field_index => Some($name::$field),)+
                    _ => None,
                }
            }

            fn from_str<S: AsRef<str>>(input: S) -> Option<$name> {
                let lowered = input.as_ref().to_ascii_lowercase();

                match lowered.as_str() {
                    $($field_str => Some($name::$field),)+
                    _ => None,
                }
            }
        }
    };
}

// Convenience macro to implement the EntryValues trait without having to specify
// struct fields multiple times
macro_rules! impl_entryvalues {
    ($struct:ident, $($field:ident($val_name:ident): $xml_name:expr => $xml_val:expr,)+) => {
        impl EntryValues for $struct {
            #[doc(hidden)]
            fn add_changed_values(&self, xml_elem: &mut Element) {
                $(if self.$field.changed {
                    let $val_name = &self.$field.value;

                    let mut elem = Element::bare($xml_name);
                    elem.append_text_node($xml_val);
                    xml_elem.append_child(elem);
                })+
            }

            #[doc(hidden)]
            fn reset_changed_fields(&mut self) {
                $(self.$field.changed = false;)+
            }
        }
    };
}

#[cfg(feature = "anime")]
pub mod anime;
#[cfg(feature = "manga")]
pub mod manga;

#[derive(Fail, Debug)]
pub enum ListError {
    #[fail(display = "no user info found")]
    NoUserInfoFound,
}

/// This struct allows you to add, update, delete, and read entries to / from a user's list.
/// 
/// The `E` type parameter dictates what type of list is will be modified when performing operations.
/// 
/// # Examples
/// 
/// ```no_run
/// use mal::MAL;
/// use mal::list::List;
/// use mal::list::anime::{AnimeEntry, AnimeValues, WatchStatus};
/// 
/// // Create a new MAL instance
/// let mal = MAL::new("username", "password");
/// 
/// // Create a new List that will operate on a user's anime list.
/// // (note that you can also just call mal.anime_list() here, which does the same thing)
/// let anime_list = List::<AnimeEntry>::new(&mal);
/// 
/// // Create new anime entry values
/// let mut values = AnimeValues::new();
/// 
/// // Set the watched episode count to 25, and status to completed
/// values.set_watched_episodes(25)
///       .set_status(WatchStatus::Completed);
/// 
/// // Add the anime with ID 4224 (Toradora) to a user's anime list with the values set above
/// anime_list.add_id(4224, &mut values).unwrap();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct List<'a, E: ListEntry> {
    /// A reference to the [`MAL`] instance used to perform operations on a user's list.
    /// 
    /// [`MAL`]: ../struct.MAL.html
    pub mal: &'a MAL,
    _list_entry: PhantomData<E>,
}

impl<'a, E: ListEntry> List<'a, E> {
    /// Creates a new `List` instance for performing operations on a user's list.
    #[inline]
    pub fn new(mal: &'a MAL) -> List<'a, E> {
        List {
            mal,
            _list_entry: PhantomData,
        }
    }

    /// Requests and parses all entries on a user's list.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mal::MAL;
    /// 
    /// // Create a new MAL instance
    /// let mal = MAL::new("username", "password");
    /// 
    /// // Read the user's anime list
    /// let list = mal.anime_list().read().unwrap();
    /// 
    /// println!("{:?}", list.user_info);
    /// println!("{:?}", list.entries);
    /// ```
    pub fn read(&self) -> Result<ListEntries<E>, MALError> {
        let resp = Request::List(&self.mal.username, E::list_type())
            .send(self.mal)
            .map_err(MALError::Request)?;

        let root: Element = resp
            .parse()
            .map_err(|e| MALError::Internal(SyncFailure::new(e).into()))?;

        let mut children = root.children();

        let user_info = {
            let elem = children
                .next()
                .ok_or_else(|| MALError::Internal(ListError::NoUserInfoFound.into()))?;

            UserInfo::parse(elem).map_err(MALError::Internal)?
        };

        let mut entries = Vec::new();

        for child in children {
            let entry = E::parse(child).map_err(MALError::Internal)?;
            entries.push(entry);
        }

        let entries = ListEntries {
            user_info,
            entries,
        };

        Ok(entries)
    }

    /// Adds an entry to a user's list.
    /// 
    /// If the entry is already on a user's list, nothing will happen.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mal::MAL;
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
    pub fn add(&self, entry: &mut E) -> Result<(), MALError> {
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
    /// use mal::MAL;
    /// use mal::list::anime::{AnimeValues, WatchStatus};
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
    pub fn add_id(&self, id: u32, values: &mut E::Values) -> Result<(), MALError> {
        let body = values.generate_xml().map_err(MALError::Internal)?;

        Request::Add(id, E::list_type(), &body)
            .send(self.mal)
            .map_err(MALError::Request)?;

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
    /// use mal::MAL;
    /// use mal::list::anime::WatchStatus;
    /// 
    /// // Create a new MAL instance
    /// let mal = MAL::new("username", "password");
    /// 
    /// // Get a handle to the user's anime list
    /// let anime_list = mal.anime_list();
    /// 
    /// // Read the user's anime list
    /// let list = anime_list.read().unwrap();
    /// 
    /// // Find Toradora in the list entries
    /// let mut toradora = list
    ///     .entries
    ///     .into_iter()
    ///     .find(|e| e.series_info.id == 4224).unwrap();
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
    pub fn update(&self, entry: &mut E) -> Result<(), MALError> {
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
    /// use mal::MAL;
    /// use mal::list::anime::{AnimeValues, WatchStatus};
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
    pub fn update_id(&self, id: u32, values: &mut E::Values) -> Result<(), MALError> {
        let body = values.generate_xml().map_err(MALError::Internal)?;

        Request::Update(id, E::list_type(), &body)
            .send(self.mal)
            .map_err(MALError::Request)?;

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
    /// use mal::MAL;
    /// use mal::list::anime::WatchStatus;
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
    /// // Read the user's anime list
    /// let list = anime_list.read().unwrap();
    /// 
    /// // Find Toradora in the list entries
    /// let toradora = list
    ///     .entries
    ///     .into_iter()
    ///     .find(|e| e.series_info.id == 4224).unwrap();
    /// 
    /// // Delete Toradora from the user's anime list
    /// anime_list.delete(&toradora).unwrap();
    /// ```
    #[inline]
    pub fn delete(&self, entry: &E) -> Result<(), MALError> {
        self.delete_id(entry.id())
    }

    /// Removes an entry from a user's list by its id.
    /// 
    /// If the entry isn't already on a user's list, nothing will happen.
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// use mal::MAL;
    /// use mal::list::anime::WatchStatus;
    /// 
    /// // Create a new MAL instance
    /// let mal = MAL::new("username", "password");
    /// 
    /// // Delete the anime with the id of 4224 (Toradora) from the user's anime list
    /// mal.anime_list().delete_id(4224).unwrap();
    /// ```
    #[inline]
    pub fn delete_id(&self, id: u32) -> Result<(), MALError> {
        Request::Delete(id, E::list_type())
            .send(self.mal)
            .map_err(MALError::Request)?;
        
        Ok(())
    }
}

/// Contains the results from parsing a user's list.
#[derive(Debug)]
pub struct ListEntries<E: ListEntry> {
    /// General list statistics and info about the user.
    pub user_info: E::UserInfo,
    /// The list's entries.
    pub entries: Vec<E>,
}

/// Used for types that contain basic series information.
pub trait SeriesInfo where Self: Sized {
    #[doc(hidden)]
    fn parse_search_result(xml_elem: &Element) -> Result<Self, Error>;

    #[doc(hidden)]
    fn list_type() -> ListType;
}

/// Represents an entry on a user's list.
pub trait ListEntry where Self: Sized {
    type Values: EntryValues;
    type UserInfo: UserInfo;

    #[doc(hidden)]
    fn parse(xml_elem: &Element) -> Result<Self, Error>;

    #[doc(hidden)]
    fn values_mut(&mut self) -> &mut Self::Values;

    #[doc(hidden)]
    fn set_last_updated_time(&mut self);

    #[doc(hidden)]
    fn id(&self) -> u32;

    #[doc(hidden)]
    fn list_type() -> ListType;
}

/// Represents values on a user's list that can be set.
pub trait EntryValues {
    #[doc(hidden)]
    fn generate_xml(&self) -> Result<String, Error> {
        let mut entry = Element::bare("entry");
        self.add_changed_values(&mut entry);

        let mut buffer = Vec::new();
        entry.write_to(&mut buffer).map_err(SyncFailure::new)?;

        Ok(String::from_utf8(buffer)?)
    }

    #[doc(hidden)]
    fn add_changed_values(&self, xml_elem: &mut Element);

    #[doc(hidden)]
    fn reset_changed_fields(&mut self);
}

/// Represents info about a user's list.
pub trait UserInfo where Self: Sized {
    #[doc(hidden)]
    fn parse(xml_elem: &Element) -> Result<Self, Error>;
}

#[derive(Debug, Default, Clone)]
struct ChangeTracker<T: Debug + Default + Clone> {
    value: T,
    changed: bool,
}

impl<T: Debug + Default + Clone> ChangeTracker<T> {
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

impl<T: Debug + Default + Clone> From<T> for ChangeTracker<T> {
    fn from(value: T) -> Self {
        ChangeTracker::new(value)
    }
}
