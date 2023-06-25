use std::{
    fmt::Display,
    sync::{RwLock, RwLockReadGuard},
};

use ahash::RandomState;

thread_local! {
    static IMPORT_CACHE_HASH: RwLock<String> = RwLock::new(String::from("0"));
    static RANDOM_STATE: RandomState = RandomState::with_seed(2304);
}

/// Generate an import statement with version hash for prevent import caching.
/// The hash is only used with relative imports.
pub fn import<T: Display, F: Display>(t: T, filename: F) -> String {
    let filename = import_filename(filename);

    format!("import {t} from \"{filename}\";")
}

/// Generate a filename with cache hash.
/// Note: Don't use quotes, the output is clean
pub fn import_filename<F: Display>(filename: F) -> String {
    let filename = filename.to_string();

    return if &filename[0..1] == "." {
        with_cache_hash(|hash| format!("{filename}?cache_hash={hash}"))
    } else {
        filename
    };
}

/// Generate and set a new global hash for import caching
pub fn new_import_hash() {
    let h = RANDOM_STATE.with(|r| with_cache_hash(|hash| r.hash_one(hash.to_string())));
    let h = format!("{h}");
    IMPORT_CACHE_HASH.with(|cache_hash| {
        let mut cache_hash = cache_hash.write().unwrap();
        *cache_hash = h;
    });
}

fn with_cache_hash<F, R>(f: F) -> R
where
    F: FnOnce(RwLockReadGuard<'_, String>) -> R,
{
    IMPORT_CACHE_HASH.with(|i| f(i.read().unwrap()))
}
