# mal-rs [![Crates.io](https://img.shields.io/crates/v/mal.svg)](https://crates.io/crates/mal) [![Documentation](https://docs.rs/mal/badge.svg)](https://docs.rs/mal)
**NOTE**: Since the MyAnimeList API was suddenly shut down, this library does not currently work. If/when the API is restored, this library will be updated to support it unless the new API does not require a full-scale library to be used. All future development will happen here: https://gitlab.com/Acizza/mal-rs

The purpose of this library is to provide high-level access to the [MyAnimeList](https://myanimelist.net) API.

At the time of writing, all features of the API are implemented, which include:
* Adding, updating, removing, and reading entries from a user's anime and manga list
* Searching for anime and manga by name
* Getting misc. info and statistics from a user's anime and manga list
* Verifying user credentials

# Usage
By default, the library builds with support to work with both anime and manga.
If you need to search for / perform operations on both the user's anime and manga list, you can simply add `mal` as a dependency to your `Cargo.toml` file:
```toml
[dependencies]
mal = "0.8"
```

If you only need to work with just one type of list, you should use the following feature gates to reduce the final binary size and compilation time:

If you only need to search for anime / work with anime on a user's list, use the `anime` feature:
```toml
[dependencies.mal]
version = "0.8"
default-features = false

features = ["anime"]
```

If you only need to search for manga / work with manga on a user's list, use the `manga` feature:
```toml
[dependencies.mal]
version = "0.8"
default-features = false

features = ["manga"]
```

# Example

The following will update an existing anime on a user's list:
```rust
extern crate mal;

use mal::MAL;
use mal::list::Status;

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
            .set_status(Status::Completed);

    // Update the anime on the user's list
    mal.anime_list().update(&mut toradora).unwrap();
}
```

For more examples, see the docs on [docs.rs](https://docs.rs/mal).
