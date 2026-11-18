//! Brave browsing history, searched across every profile.
//!
//! Each profile keeps its history in a SQLite database
//! (`<user data dir>/<profile dir>/History`). Brave holds a lock on it while it
//! runs, so it is opened read-only with `immutable=1` through the system
//! `sqlite3` CLI: no copy, no lock contention. The search terms are pushed into
//! the SQL query (`LIKE`) so the whole history is searchable, not just the most
//! recent entries.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::profiles::Profile;
use crate::runner::CommandRunner;

/// Path of the system `sqlite3` binary.
pub const SQLITE: &str = "/usr/bin/sqlite3";

/// Rows fetched per profile and shown in total.
pub const LIMIT: usize = 50;

/// Seconds between 1601-01-01 (Chromium's epoch) and 1970-01-01.
const CHROMIUM_EPOCH_OFFSET_SECS: i64 = 11_644_473_600;

/// One history entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub url: String,
    pub title: String,
    /// Last visit, in seconds since the Unix epoch.
    pub last_visit: i64,
    /// Directory of the profile the entry comes from.
    pub profile_dir: String,
    /// Display name of that profile.
    pub profile_name: String,
}

impl Entry {
    /// The title, or the URL for pages that had none.
    pub fn display_title(&self) -> &str {
        if self.title.trim().is_empty() {
            return &self.url;
        }

        &self.title
    }
}

#[derive(Debug, Deserialize)]
struct Row {
    url: String,
    #[serde(default)]
    title: Option<String>,
    last_visit_time: i64,
}

/// Build the SQL listing the most recent visible URLs whose URL or title
/// contains every term (case-insensitive for ASCII, like SQLite's `LIKE`).
pub fn build_sql(query: &str, limit: usize) -> String {
    let conditions: Vec<String> = query
        .split_whitespace()
        .map(|term| {
            let pattern = like_pattern(term);

            format!("(url LIKE '{pattern}' ESCAPE '\\' OR title LIKE '{pattern}' ESCAPE '\\')")
        })
        .collect();

    let mut sql = String::from("SELECT url, title, last_visit_time FROM urls WHERE hidden = 0");

    for condition in conditions {
        sql.push_str(" AND ");
        sql.push_str(&condition);
    }

    sql.push_str(&format!(" ORDER BY last_visit_time DESC LIMIT {limit}"));

    sql
}

/// `%term%`, with `LIKE` wildcards escaped and quotes doubled for a SQL string
/// literal.
fn like_pattern(term: &str) -> String {
    let escaped = term
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
        .replace('\'', "''");

    format!("%{escaped}%")
}

/// SQLite URI opening `path` read-only and immutable (no locking, no journal).
pub fn database_uri(path: &Path) -> String {
    let mut encoded = String::new();

    for c in path.to_string_lossy().chars() {
        match c {
            '%' => encoded.push_str("%25"),
            ' ' => encoded.push_str("%20"),
            '?' => encoded.push_str("%3F"),
            '#' => encoded.push_str("%23"),
            _ => encoded.push(c),
        }
    }

    format!("file:{encoded}?immutable=1")
}

/// Convert a Chromium timestamp (microseconds since 1601) to Unix seconds.
pub fn chromium_to_unix(micros: i64) -> i64 {
    micros / 1_000_000 - CHROMIUM_EPOCH_OFFSET_SECS
}

/// Parse `sqlite3 -json` output. An empty result prints nothing at all.
pub fn parse_rows(json: &str, profile: &Profile) -> Result<Vec<Entry>> {
    if json.trim().is_empty() {
        return Ok(Vec::new());
    }

    let rows: Vec<Row> =
        serde_json::from_str(json).context("unexpected output from sqlite3 on the history")?;

    Ok(rows
        .into_iter()
        .map(|row| Entry {
            url: row.url,
            title: row.title.unwrap_or_default(),
            last_visit: chromium_to_unix(row.last_visit_time),
            profile_dir: profile.dir.clone(),
            profile_name: profile.name.clone(),
        })
        .collect())
}

/// Merge the per-profile results: one entry per URL (its most recent visit),
/// most recent first, at most `limit`.
pub fn merge(entries: Vec<Entry>, limit: usize) -> Vec<Entry> {
    let mut latest: HashMap<String, Entry> = HashMap::new();

    for entry in entries {
        match latest.get(&entry.url) {
            Some(existing) if existing.last_visit >= entry.last_visit => {}
            _ => {
                latest.insert(entry.url.clone(), entry);
            }
        }
    }

    let mut merged: Vec<Entry> = latest.into_values().collect();
    merged.sort_by(|a, b| {
        b.last_visit
            .cmp(&a.last_visit)
            .then_with(|| a.url.cmp(&b.url))
    });
    merged.truncate(limit);

    merged
}

/// Search the history of every profile that has a `History` database.
///
/// A profile whose database cannot be read is skipped, so one broken profile
/// does not hide the others; the error is only returned when every profile
/// failed.
pub fn search(
    runner: &dyn CommandRunner,
    user_data_dir: &Path,
    profiles: &[Profile],
    query: &str,
) -> Result<Vec<Entry>> {
    let sql = build_sql(query, LIMIT);
    let mut entries = Vec::new();
    let mut last_error = None;
    let mut succeeded = 0;

    for profile in profiles {
        let database = user_data_dir.join(&profile.dir).join("History");

        if !database.is_file() {
            continue;
        }

        let uri = database_uri(&database);
        let result = runner
            .run(SQLITE, &["-readonly", "-json", &uri, &sql])
            .and_then(|json| parse_rows(&json, profile));

        match result {
            Ok(rows) => {
                succeeded += 1;
                entries.extend(rows);
            }
            Err(err) => last_error = Some(err),
        }
    }

    if succeeded == 0 {
        if let Some(err) = last_error {
            return Err(err);
        }
    }

    Ok(merge(entries, LIMIT))
}
