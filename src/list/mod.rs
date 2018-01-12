use failure::Error;
use std::fmt::Debug;

#[cfg(feature = "anime-list")]
pub mod anime;

/// Contains methods that perform common operations on a user's anime / manga list.
pub trait List {
    /// Type representing an entry on a user's anime / manga list.
    type Entry;

    /// Reads all of the entries on a user's anime / manga list.
    fn read_entries(&self) -> Result<Vec<Self::Entry>, Error>;

    /// Adds an entry to a user's anime / manga list.
    fn add(&self, entry: &Self::Entry) -> Result<(), Error>;
    /// Updates an entry on a user's anime / manga list.
    fn update(&self, entry: &mut Self::Entry) -> Result<(), Error>;
    /// Deletes an entry from a user's anime / manga list.
    fn delete(&self, entry: &Self::Entry) -> Result<(), Error>;
    /// Deletes an entry by its id from a user's anime / manga list.
    fn delete_id(&self, id: u32) -> Result<(), Error>;
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
