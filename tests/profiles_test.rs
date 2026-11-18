//! Tests for reading Brave profiles from `Local State`.

use std::fs;

use alfred_brave::profiles;

const LOCAL_STATE: &str = r#"{
  "profile": {
    "last_used": "Profile 1",
    "info_cache": {
      "Default": { "name": "Personal", "active_time": 1700000000.5 },
      "Profile 1": { "name": "Work", "active_time": 1800000000.0 },
      "Profile 2": { "name": "  ", "active_time": 1600000000.0 }
    }
  }
}"#;

#[test]
fn profiles_are_ordered_by_most_recent_activity() {
    let parsed = profiles::parse_local_state(LOCAL_STATE);
    let dirs: Vec<&str> = parsed.iter().map(|p| p.dir.as_str()).collect();

    assert_eq!(dirs, ["Profile 1", "Default", "Profile 2"]);
    assert_eq!(parsed[0].name, "Work");
}

#[test]
fn blank_name_falls_back_to_the_directory() {
    let parsed = profiles::parse_local_state(LOCAL_STATE);

    assert_eq!(parsed[2].name, "Profile 2");
}

#[test]
fn malformed_or_unexpected_state_yields_no_profile() {
    assert!(profiles::parse_local_state("not json").is_empty());
    assert!(profiles::parse_local_state(r#"{"profile": {}}"#).is_empty());
}

#[test]
fn missing_local_state_yields_no_profile() {
    let dir = std::env::temp_dir().join("alfred-brave-no-such-dir");

    assert!(profiles::load(&dir).is_empty());
}

#[test]
fn load_reads_local_state_from_the_user_data_dir() {
    let dir = std::env::temp_dir().join(format!("alfred-brave-profiles-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("Local State"), LOCAL_STATE).unwrap();

    let loaded = profiles::load(&dir);
    fs::remove_dir_all(&dir).unwrap();

    assert_eq!(loaded.len(), 3);
}

#[test]
fn filter_matches_name_or_directory_case_insensitively() {
    let parsed = profiles::parse_local_state(LOCAL_STATE);

    let by_name = profiles::filter(parsed.clone(), "WORK");
    assert_eq!(by_name.len(), 1);
    assert_eq!(by_name[0].dir, "Profile 1");

    let by_dir = profiles::filter(parsed, "default");
    assert_eq!(by_dir.len(), 1);
    assert_eq!(by_dir[0].name, "Personal");
}
