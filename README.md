# mal-rs [![Crates.io](https://img.shields.io/crates/v/mal.svg)](https://crates.io/crates/mal) [![Documentation](https://docs.rs/mal/badge.svg)](https://docs.rs/mal)
The purpose of this library is to provide high-level access to the [MyAnimeList](https://myanimelist.net) API. 

At the time of writing, all features of the API are implemented, which include:
* Adding, updating, removing, and reading entries from a user's anime and manga list
* Searching for anime and manga by name
* Getting misc. info and statistics from a user's anime and manga list
* Verifying user credentials

# Usage
By default, the library builds with support to access a user's anime and manga list.
If you plan on performing operations on both, you can simply add `mal` as a dependency to your `Cargo.toml` file:
```toml
[dependencies]
mal = "0.5"
```

If you only need to access one type of list (or none at all), you should use the following feature gates to reduce the final binary size and compilation time:

If you only need access to the user's anime list, use the `anime-list` feature:
```toml
[dependencies.mal]
version = "0.5"
default-features = false

features = ["anime-list"]
```

If you only need access to a user's manga list, use the `manga-list` feature:
```toml
[dependencies.mal]
version = "0.5"
default-features = false

features = ["manga-list"]
```

Or, if you don't need to access either, just disable the default features:
```toml
[dependencies.mal]
version = "0.5"
default-features = false
```

# Example

The following will update an existing anime on a user's list:
```rust
extern crate mal;

use mal::MAL;
use mal::list::anime::WatchStatus;

fn main() {
    // Create a new MAL instance
    let mal = MAL::new("username", "password");

    // Read the user's anime list
    let list = mal.anime_list().read().unwrap();

    // Find Toradora in the list entries
    let mut toradora = list
        .entries
        .into_iter()
        .find(|e| e.series_info.id == 4224)
        .unwrap();

    // Set new values for the list entry
    // In this case, the episode count will be updated to 25, the score will be set to 10, and the status will be set to completed
    toradora.values
            .set_watched_episodes(25)
            .set_score(10)
            .set_status(WatchStatus::Completed);

    // Update the anime on the user's list
    mal.anime_list().update(&mut toradora).unwrap();
}
```

For more examples, see the docs on [docs.rs](https://docs.rs/mal).