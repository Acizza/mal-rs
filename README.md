# mal-rs [![Crates.io](https://img.shields.io/crates/v/mal.svg)](https://crates.io/crates/mal) [![Documentation](https://docs.rs/mal/badge.svg)](https://docs.rs/mal)
The purpose of this library is to provide high-level access to the MyAnimeList API. Currently you can only interact with a user's anime list, but the ability to interact with the user's manga list is planned for a future version.

Please note that while most data fields are parsed from the API, there are some that are purposely ignored because they are only available when making certain types of requests.

# Examples

The following will update an existing anime on a user's list, but the code to add / delete an anime is similar:
```rust
extern crate mal;

use mal::{MAL, SeriesInfo};
use mal::list::{AnimeList, ListEntry, Status};

fn main() {
    // Create a new MAL instance
    let mal = MAL::new("username", "password");

    // Create a new AnimeList instance
    let anime_list = AnimeList::new(&mal);

    // Get and parse all of the list entries
    let entries = anime_list.read_entries().unwrap();

    // Find Toradora in the list entries
    let mut toradora_entry = entries.into_iter().find(|e| e.series_info.id == 4224).unwrap();

    // Set new values for the list entry
    // In this case, the episode count will be updated to 25, the score will be set to 10, and the status will be set to completed
    toradora_entry.set_watched_episodes(25)
                .set_score(10)
                .set_status(Status::Completed);

    // Update the anime on the user's list
    anime_list.update(&mut toradora_entry).unwrap();
}
```