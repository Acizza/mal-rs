use failure::Error;

#[cfg(feature="anime-list")]
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
