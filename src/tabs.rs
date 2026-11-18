//! Open Brave tabs and how the `bwt` query filters them.

use serde::Deserialize;

/// One open tab, as reported by Brave.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    pub window_id: String,
    pub tab_id: String,
    pub title: String,
    pub url: String,
    /// Whether the tab lives in a private (incognito) window.
    pub private: bool,
    /// Whether this is the tab currently shown in the front window.
    pub active: bool,
}

impl Tab {
    /// The title, or the URL for tabs that have none yet.
    pub fn display_title(&self) -> &str {
        if self.title.trim().is_empty() {
            return &self.url;
        }

        &self.title
    }
}

/// Keep the tabs whose title or URL contains every whitespace-separated term
/// of `query` (case-insensitive substring), preserving Brave's order.
pub fn filter(tabs: Vec<Tab>, query: &str) -> Vec<Tab> {
    let terms = lowercase_terms(query);

    tabs.into_iter()
        .filter(|tab| matches_all(&terms, &[&tab.title, &tab.url]))
        .collect()
}

/// Split `query` into lowercase terms.
pub fn lowercase_terms(query: &str) -> Vec<String> {
    query.split_whitespace().map(str::to_lowercase).collect()
}

/// Whether every term is a substring of at least one of `fields`.
pub fn matches_all(terms: &[String], fields: &[&str]) -> bool {
    let haystack = fields
        .iter()
        .map(|field| field.to_lowercase())
        .collect::<Vec<_>>()
        .join("\n");

    terms.iter().all(|term| haystack.contains(term.as_str()))
}
