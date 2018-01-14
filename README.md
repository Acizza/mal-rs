# mal-rs [![Crates.io](https://img.shields.io/crates/v/mal.svg)](https://crates.io/crates/mal) [![Documentation](https://docs.rs/mal/badge.svg)](https://docs.rs/mal)
The purpose of this library is to provide high-level access to the MyAnimeList API. It currently allows you to search for anime / manga, verify user credentials, and add / update / delete anime and manga from a user's list.

# Usage
By default, the library builds with support to access a user's anime and manga list.
If you plan on performing operations on both, you can simply add `mal` as a dependency to your `Cargo.toml` file:
```toml
[dependencies]
mal = "0.4"
```

If you only need to access one type of list (or none at all), you should use the following feature gates to reduce the final binary size and compilation time:

If you only need access to the user's anime list, use the `anime-list` feature:
```toml
[dependencies.mal]
version = "0.4"
default-features = false

features = ["anime-list"]
```

If you only need access to a user's manga list, use the `manga-list` feature:
```toml
[dependencies.mal]
version = "0.4"
default-features = false

features = ["manga-list"]
```

Or, if you don't need to access either, just disable the default features:
```toml
[dependencies.mal]
version = "0.4"
default-features = false
```

# Example

The following will update an existing anime on a user's list, but the code to add / delete an anime is similar:
```rust
extern crate mal;

use mal::MAL;
use mal::list::List;
use mal::list::anime::WatchStatus;

fn main() {
    // Create a new MAL instance
    let mal = MAL::new("username", "password");

    // Get a handle to the user's anime list
    let anime_list = mal.anime_list();

    // Get and parse all of the list entries
    let entries = anime_list.read_entries().unwrap();

    // Find Toradora in the list entries
    let mut toradora_entry = entries.into_iter().find(|e| e.series_info.id == 4224).unwrap();

    // Set new values for the list entry
    // In this case, the episode count will be updated to 25, the score will be set to 10, and the status will be set to completed
    toradora_entry.set_watched_episodes(25)
                  .set_score(10)
                  .set_status(WatchStatus::Completed);

    // Update the anime on the user's list
    anime_list.update(&mut toradora_entry).unwrap();
}
```