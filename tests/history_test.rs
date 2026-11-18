//! Tests for searching Brave's history databases.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use alfred_brave::history::{self, Entry, SQLITE};
use alfred_brave::profiles::Profile;
use common::FakeRunner;

fn profile(dir: &str, name: &str) -> Profile {
    Profile {
        dir: dir.to_owned(),
        name: name.to_owned(),
        active_time: 0.0,
    }
}

fn entry(url: &str, last_visit: i64, dir: &str) -> Entry {
    Entry {
        url: url.to_owned(),
        title: String::new(),
        last_visit,
        profile_dir: dir.to_owned(),
        profile_name: dir.to_owned(),
    }
}

/// A fake user data dir where each given profile has an (empty) History file.
fn user_data_dir(test: &str, dirs: &[&str]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("alfred-brave-{test}-{}", std::process::id()));

    for dir in dirs {
        fs::create_dir_all(root.join(dir)).unwrap();
        fs::write(root.join(dir).join("History"), "").unwrap();
    }

    root
}

#[test]
fn sql_without_terms_lists_recent_visible_urls() {
    assert_eq!(
        history::build_sql("  ", 10),
        "SELECT url, title, last_visit_time FROM urls WHERE hidden = 0 \
         ORDER BY last_visit_time DESC LIMIT 10"
    );
}

#[test]
fn sql_requires_every_term_in_url_or_title() {
    let sql = history::build_sql("rust docs", 5);

    assert!(sql.contains(
        "AND (url LIKE '%rust%' ESCAPE '\\' OR title LIKE '%rust%' ESCAPE '\\') \
         AND (url LIKE '%docs%' ESCAPE '\\' OR title LIKE '%docs%' ESCAPE '\\')"
    ));
}

#[test]
fn sql_escapes_quotes_and_like_wildcards() {
    let sql = history::build_sql("it's 100%_", 5);

    assert!(sql.contains("'%it''s%'"));
    assert!(sql.contains("'%100\\%\\_%'"));
}

#[test]
fn database_uri_is_percent_encoded_and_immutable() {
    let uri = history::database_uri(Path::new("/Users/me/Application Support/Default/History"));

    assert_eq!(
        uri,
        "file:/Users/me/Application%20Support/Default/History?immutable=1"
    );
}

#[test]
fn chromium_timestamps_convert_to_unix_seconds() {
    // 2024-01-01T00:00:00Z
    assert_eq!(
        history::chromium_to_unix(13_348_540_800_000_000),
        1_704_067_200
    );
}

#[test]
fn parse_rows_handles_empty_output_and_null_titles() {
    let work = profile("Profile 1", "Work");

    assert!(history::parse_rows("", &work).unwrap().is_empty());

    let rows = history::parse_rows(
        r#"[{"url":"https://a.test","title":null,"last_visit_time":13348281600000000}]"#,
        &work,
    )
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].title, "");
    assert_eq!(rows[0].display_title(), "https://a.test");
    assert_eq!(rows[0].profile_name, "Work");
}

#[test]
fn merge_keeps_the_latest_visit_per_url_most_recent_first() {
    let merged = history::merge(
        vec![
            entry("https://a.test", 10, "Default"),
            entry("https://b.test", 30, "Default"),
            entry("https://a.test", 20, "Profile 1"),
        ],
        10,
    );

    let urls: Vec<(&str, &str)> = merged
        .iter()
        .map(|e| (e.url.as_str(), e.profile_dir.as_str()))
        .collect();

    assert_eq!(
        urls,
        [
            ("https://b.test", "Default"),
            ("https://a.test", "Profile 1")
        ]
    );
}

#[test]
fn merge_truncates_to_the_limit() {
    let merged = history::merge(
        vec![
            entry("https://a.test", 1, "D"),
            entry("https://b.test", 2, "D"),
        ],
        1,
    );

    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].url, "https://b.test");
}

#[test]
fn search_queries_each_profile_with_a_history_database() {
    let root = user_data_dir("search", &["Default"]);
    let runner = FakeRunner::new().with_success(
        SQLITE,
        r#"[{"url":"https://a.test","title":"A","last_visit_time":13348281600000000}]"#,
    );
    let profiles = [profile("Default", "Personal"), profile("Profile 9", "Gone")];

    let entries = history::search(&runner, &root, &profiles, "a").unwrap();
    fs::remove_dir_all(&root).unwrap();

    // "Profile 9" has no History file and is skipped.
    let calls = runner.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].args[0], "-readonly");
    assert_eq!(calls[0].args[1], "-json");
    assert!(calls[0].args[2].ends_with("/Default/History?immutable=1"));
    assert!(calls[0].args[3].contains("LIKE '%a%'"));

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].profile_name, "Personal");
}

#[test]
fn search_skips_a_broken_profile_when_another_succeeds() {
    let root = user_data_dir("partial", &["Default", "Profile 1"]);
    let runner = FakeRunner::new()
        .with_failure(SQLITE, "database is locked")
        .with_success(
            SQLITE,
            r#"[{"url":"https://a.test","title":"A","last_visit_time":13348281600000000}]"#,
        );
    let profiles = [profile("Default", "Personal"), profile("Profile 1", "Work")];

    let entries = history::search(&runner, &root, &profiles, "").unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].profile_dir, "Profile 1");
}

#[test]
fn search_fails_when_every_profile_fails() {
    let root = user_data_dir("failing", &["Default"]);
    let runner = FakeRunner::new().with_failure(SQLITE, "database is locked");

    let result = history::search(&runner, &root, &[profile("Default", "Personal")], "");
    fs::remove_dir_all(&root).unwrap();

    assert!(result
        .unwrap_err()
        .to_string()
        .contains("database is locked"));
}
