//! Tests for tab parsing (Brave's JXA output), filtering and the Brave calls.

mod common;

use alfred_brave::brave::{self, WindowMode, APP_NAME, OPEN, OSASCRIPT};
use alfred_brave::tabs::{self, Tab};
use common::FakeRunner;

fn tab(title: &str, url: &str) -> Tab {
    Tab {
        window_id: "1".to_owned(),
        tab_id: "2".to_owned(),
        title: title.to_owned(),
        url: url.to_owned(),
        private: false,
        active: false,
    }
}

const TABS_JSON: &str = r#"[
  {"windowId":"10","tabId":"11","title":"Rust docs","url":"https://doc.rust-lang.org/","private":false,"active":true},
  {"windowId":"20","tabId":"21","title":"","url":"https://example.com/","private":true,"active":false}
]"#;

#[test]
fn parse_tabs_reads_the_jxa_output() {
    let parsed = brave::parse_tabs(TABS_JSON).unwrap().unwrap();

    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].window_id, "10");
    assert!(parsed[0].active);
    assert!(parsed[1].private);
    assert_eq!(parsed[1].display_title(), "https://example.com/");
}

#[test]
fn parse_tabs_null_means_brave_is_not_running() {
    assert_eq!(brave::parse_tabs("null\n").unwrap(), None);
    assert_eq!(brave::parse_current_tab("null").unwrap(), None);
}

#[test]
fn parse_tabs_rejects_garbage() {
    assert!(brave::parse_tabs("execution error").is_err());
}

#[test]
fn filter_requires_every_term_in_title_or_url() {
    let all = vec![
        tab("Rust docs", "https://doc.rust-lang.org/"),
        tab("Brave Search", "https://search.brave.com/"),
    ];

    let matching = tabs::filter(all.clone(), "RUST lang");
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0].title, "Rust docs");

    assert_eq!(tabs::filter(all.clone(), "").len(), 2);
    assert!(tabs::filter(all, "rust brave").is_empty());
}

#[test]
fn list_tabs_runs_jxa_and_parses_its_output() {
    let runner = FakeRunner::new().with_success(OSASCRIPT, TABS_JSON);

    let listed = brave::list_tabs(&runner).unwrap().unwrap();

    assert_eq!(listed.len(), 2);
    let call = &runner.calls()[0];
    assert_eq!(call.args[..3], ["-l", "JavaScript", "-e"]);
    assert!(call.args[3].contains("Application(\"Brave Browser\")"));
}

#[test]
fn focus_tab_passes_ids_as_arguments() {
    let runner = FakeRunner::new().with_success(OSASCRIPT, "");

    brave::focus_tab(&runner, "10", "11").unwrap();

    assert_eq!(FakeRunner::jxa_args(&runner.calls()[0]), ["10", "11"]);
}

#[test]
fn new_window_passes_mode_url_and_count() {
    let runner = FakeRunner::new().with_success(OSASCRIPT, "");

    brave::new_window(&runner, WindowMode::Private, Some("https://a.test"), 1).unwrap();
    brave::new_window(&runner, WindowMode::Normal, None, 3).unwrap();

    let calls = runner.calls();
    assert_eq!(
        FakeRunner::jxa_args(&calls[0]),
        ["incognito", "https://a.test", "1"]
    );
    assert_eq!(FakeRunner::jxa_args(&calls[1]), ["normal", "", "3"]);
}

#[test]
fn close_tabs_with_url_returns_the_count() {
    let runner = FakeRunner::new().with_success(OSASCRIPT, "2\n");

    assert_eq!(
        brave::close_tabs_with_url(&runner, "https://a.test").unwrap(),
        2
    );
}

#[test]
fn open_in_profile_with_a_url_opens_a_tab_in_the_current_window() {
    let runner = FakeRunner::new().with_success(OPEN, "");

    brave::open_in_profile(&runner, "Profile 1", Some("https://a.test")).unwrap();

    // No `--new-window`: Brave adds a tab to the profile's current window.
    assert_eq!(
        runner.calls()[0].args,
        [
            "-na",
            APP_NAME,
            "--args",
            "--profile-directory=Profile 1",
            "https://a.test"
        ]
    );
}

#[test]
fn open_in_profile_without_a_url_opens_a_blank_window() {
    let runner = FakeRunner::new().with_success(OPEN, "");

    brave::open_in_profile(&runner, "Default", None).unwrap();

    assert_eq!(
        runner.calls()[0].args,
        [
            "-na",
            APP_NAME,
            "--args",
            "--profile-directory=Default",
            "--new-window"
        ]
    );
}
