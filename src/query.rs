//! Parsing of the `bw` (new window) query.

/// Most windows `bw xN` opens at once, to keep a typo from flooding the screen.
pub const MAX_WINDOWS: u32 = 20;

/// Search engine used when the `bw` query is not a URL.
pub const SEARCH_URL: &str = "https://search.brave.com/search?q=";

/// What the `bw` query asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowRequest {
    /// Empty query: one blank window.
    Blank,
    /// `xN`: `N` blank windows (capped at [`MAX_WINDOWS`]).
    Count(u32),
    /// A URL (scheme added to bare hosts).
    Url(String),
    /// Free text: a Brave Search for `query`, loaded from `url`.
    Search { query: String, url: String },
}

/// Interpret the `bw` query.
pub fn parse_window_request(query: &str) -> WindowRequest {
    let query = query.trim();

    if query.is_empty() {
        return WindowRequest::Blank;
    }

    if let Some(count) = parse_count(query) {
        return WindowRequest::Count(count);
    }

    if is_url(query) {
        return WindowRequest::Url(to_url(query));
    }

    WindowRequest::Search {
        query: query.to_owned(),
        url: to_url(query),
    }
}

/// Whether the query is a URL or a bare host rather than free text.
pub fn is_url(query: &str) -> bool {
    let query = query.trim();

    has_scheme(query) || looks_like_host(query)
}

fn has_scheme(query: &str) -> bool {
    query.contains("://") || query.starts_with("about:") || query.starts_with("brave:")
}

/// `x3` → 3. `x0` and non-numbers are not counts.
fn parse_count(query: &str) -> Option<u32> {
    let digits = query
        .strip_prefix('x')
        .or_else(|| query.strip_prefix('X'))?;

    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let count: u32 = digits.parse().unwrap_or(MAX_WINDOWS);

    if count == 0 {
        return None;
    }

    Some(count.min(MAX_WINDOWS))
}

/// Turn the query into something Brave can load: keep URLs with a scheme,
/// add `https://` to bare hosts (`github.com/foo`, `localhost:3000`), and
/// send everything else to Brave Search.
pub fn to_url(query: &str) -> String {
    let query = query.trim();

    if has_scheme(query) {
        return query.to_owned();
    }

    if looks_like_host(query) {
        let scheme = if is_local(query) { "http" } else { "https" };

        return format!("{scheme}://{query}");
    }

    format!("{SEARCH_URL}{}", encode_query(query))
}

fn looks_like_host(query: &str) -> bool {
    if query.contains(char::is_whitespace) {
        return false;
    }

    let host = query.split(['/', '?', '#']).next().unwrap_or("");
    let host_without_port = host.split(':').next().unwrap_or("");

    is_local(query)
        || (host_without_port.contains('.')
            && !host_without_port.starts_with('.')
            && !host_without_port.ends_with('.'))
}

fn is_local(query: &str) -> bool {
    query.starts_with("localhost") || query.starts_with("127.0.0.1")
}

/// Percent-encode a search query (`application/x-www-form-urlencoded` style).
pub fn encode_query(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());

    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }

    encoded
}
