//! Markdown links for the "copy as Markdown" actions.

/// Format `[title](url)`, escaping the characters that would break the link.
/// Falls back to the URL as the link text when the title is blank.
pub fn link(title: &str, url: &str) -> String {
    let text = if title.trim().is_empty() {
        url
    } else {
        title.trim()
    };

    let text = text
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]");
    let url = url.replace('(', "%28").replace(')', "%29");

    format!("[{text}]({url})")
}
