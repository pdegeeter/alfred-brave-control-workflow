//! Tests for parsing the `bw` query.

use alfred_brave::query::{self, WindowRequest, MAX_WINDOWS, SEARCH_URL};

#[test]
fn empty_query_is_a_blank_window() {
    assert_eq!(query::parse_window_request("  "), WindowRequest::Blank);
}

#[test]
fn x_followed_by_a_number_is_a_window_count() {
    assert_eq!(query::parse_window_request("x3"), WindowRequest::Count(3));
    assert_eq!(query::parse_window_request("X2"), WindowRequest::Count(2));
}

#[test]
fn window_count_is_capped() {
    assert_eq!(
        query::parse_window_request("x999999999999"),
        WindowRequest::Count(MAX_WINDOWS)
    );
}

#[test]
fn x_without_a_positive_number_is_a_search() {
    assert!(matches!(
        query::parse_window_request("x0"),
        WindowRequest::Search { .. }
    ));
    assert!(matches!(
        query::parse_window_request("xkcd"),
        WindowRequest::Search { .. }
    ));
}

#[test]
fn url_with_a_scheme_is_kept_as_is() {
    assert_eq!(
        query::parse_window_request("http://example.com/a?b=c"),
        WindowRequest::Url("http://example.com/a?b=c".to_owned())
    );
    assert_eq!(
        query::parse_window_request("brave://settings"),
        WindowRequest::Url("brave://settings".to_owned())
    );
}

#[test]
fn bare_host_gets_https() {
    assert_eq!(
        query::parse_window_request("github.com/pdegeeter"),
        WindowRequest::Url("https://github.com/pdegeeter".to_owned())
    );
}

#[test]
fn localhost_gets_http() {
    assert_eq!(
        query::parse_window_request("localhost:3000/health"),
        WindowRequest::Url("http://localhost:3000/health".to_owned())
    );
}

#[test]
fn free_text_is_a_brave_search() {
    assert_eq!(
        query::parse_window_request("rust lifetimes & traits"),
        WindowRequest::Search {
            query: "rust lifetimes & traits".to_owned(),
            url: format!("{SEARCH_URL}rust+lifetimes+%26+traits"),
        }
    );
}

#[test]
fn trailing_dot_is_not_a_host() {
    assert!(!query::is_url("hello."));
    assert!(query::is_url("example.org"));
}

#[test]
fn encode_query_escapes_non_ascii() {
    assert_eq!(query::encode_query("café"), "caf%C3%A9");
}
