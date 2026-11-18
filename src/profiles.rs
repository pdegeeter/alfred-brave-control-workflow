//! Brave profiles, read from the browser's own `Local State` file.
//!
//! Unlike Safari, Brave keeps its profile list in a JSON file
//! (`profile.info_cache`), so no workflow configuration is needed: every
//! profile the user created shows up automatically.

use std::path::{Path, PathBuf};

use serde_json::Value;

/// Brave's user data directory, relative to `$HOME`.
pub const USER_DATA_DIR: &str = "Library/Application Support/BraveSoftware/Brave-Browser";

/// One Brave profile.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    /// Directory name under the user data dir (`Default`, `Profile 1`, …),
    /// which is what `--profile-directory` expects.
    pub dir: String,
    /// The name the user gave the profile.
    pub name: String,
    /// Last time the profile was active (seconds since the Unix epoch, may be
    /// fractional); used for ordering.
    pub active_time: f64,
}

/// `~/Library/Application Support/BraveSoftware/Brave-Browser`, if `$HOME` is
/// set.
pub fn default_user_data_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| Path::new(&home).join(USER_DATA_DIR))
}

/// Read and parse `<user_data_dir>/Local State`. A missing or malformed file
/// yields no profile rather than an error: listings must not break.
pub fn load(user_data_dir: &Path) -> Vec<Profile> {
    std::fs::read_to_string(user_data_dir.join("Local State"))
        .map(|content| parse_local_state(&content))
        .unwrap_or_default()
}

/// Extract the profiles from the content of `Local State`, most recently
/// active first (ties broken by directory name).
pub fn parse_local_state(content: &str) -> Vec<Profile> {
    let Ok(state) = serde_json::from_str::<Value>(content) else {
        return Vec::new();
    };

    let Some(cache) = state
        .get("profile")
        .and_then(|profile| profile.get("info_cache"))
        .and_then(Value::as_object)
    else {
        return Vec::new();
    };

    let mut profiles: Vec<Profile> = cache
        .iter()
        .map(|(dir, info)| Profile {
            dir: dir.clone(),
            name: info
                .get("name")
                .and_then(Value::as_str)
                .map(|name| name.trim().to_owned())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| dir.clone()),
            active_time: info
                .get("active_time")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
        })
        .collect();

    profiles.sort_by(|a, b| {
        b.active_time
            .total_cmp(&a.active_time)
            .then_with(|| a.dir.cmp(&b.dir))
    });

    profiles
}

/// Keep the profiles whose name or directory contains every query term.
pub fn filter(profiles: Vec<Profile>, query: &str) -> Vec<Profile> {
    let terms = crate::tabs::lowercase_terms(query);

    profiles
        .into_iter()
        .filter(|profile| crate::tabs::matches_all(&terms, &[&profile.name, &profile.dir]))
        .collect()
}
