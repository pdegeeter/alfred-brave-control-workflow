//! The single Run Script action every script filter item feeds into.
//!
//! Each item (and each of its modifiers) carries an `arg` of the form
//! `"<verb> <payload>"`. [`Action::parse`] turns it back into an [`Action`] and
//! [`execute`] performs it. Payload fields are ordered so that only the last
//! one may contain spaces (titles, profile directories such as `Profile 1`);
//! URLs and ids never do.

use anyhow::{anyhow, bail, Result};

use crate::brave::{self, WindowMode};
use crate::markdown;
use crate::runner::CommandRunner;

/// Keyword of the "new window" script filter, prefilled by [`Action::Edit`].
pub const WINDOW_KEYWORD: &str = "bw";

/// Bundle id of Alfred 5, which [`Action::Edit`] asks to show a query.
pub const ALFRED_BUNDLE_ID: &str = "com.runningwithcrayons.Alfred";

const CLIPBOARD_SCRIPT: &str = "on run argv\nset the clipboard to (item 1 of argv)\nend run";

const ALFRED_SEARCH_SCRIPT: &str = "on run argv\ntell application id \"com.runningwithcrayons.Alfred\" to search (item 1 of argv)\nend run";

/// Something the user asked for by actioning an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// New window, optionally on a URL.
    Window { url: Option<String> },
    /// `count` blank windows.
    Windows { count: u32 },
    /// New private window, optionally on a URL.
    Private { url: Option<String> },
    /// New tab in the front window.
    Tab { url: String },
    /// New window in a profile.
    Profile { dir: String },
    /// Open the URL of the current tab in a profile (new tab of its window).
    ProfileWithCurrentTab { dir: String },
    /// Select a tab and raise its window.
    Focus { window_id: String, tab_id: String },
    /// Copy a URL.
    Copy { url: String },
    /// Copy a Markdown link.
    CopyMarkdown { url: String, title: String },
    /// Reopen Alfred on `bw <url>` so the URL can be edited.
    Edit { url: String },
    /// Close every tab showing this URL.
    CloseUrl { url: String },
    /// Open a history entry in a new tab of its profile's current window.
    OpenInProfile { url: String, dir: String },
    /// Reopen a tab in a private window and close the original.
    MoveToPrivate {
        window_id: String,
        tab_id: String,
        url: String,
    },
}

impl Action {
    /// Encode as the `arg` of an Alfred item.
    pub fn to_arg(&self) -> String {
        match self {
            Action::Window { url } => join("window", &[url.as_deref().unwrap_or("")]),
            Action::Windows { count } => join("windows", &[&count.to_string()]),
            Action::Private { url } => join("private", &[url.as_deref().unwrap_or("")]),
            Action::Tab { url } => join("tab", &[url]),
            Action::Profile { dir } => join("profile", &[dir]),
            Action::ProfileWithCurrentTab { dir } => join("profile-tab", &[dir]),
            Action::Focus { window_id, tab_id } => {
                join("focus", &[&format!("{window_id}:{tab_id}")])
            }
            Action::Copy { url } => join("copy", &[url]),
            Action::CopyMarkdown { url, title } => join("copy-md", &[url, title]),
            Action::Edit { url } => join("edit", &[url]),
            Action::CloseUrl { url } => join("close-url", &[url]),
            Action::OpenInProfile { url, dir } => join("open", &[url, dir]),
            Action::MoveToPrivate {
                window_id,
                tab_id,
                url,
            } => join("move-private", &[&format!("{window_id}:{tab_id}"), url]),
        }
    }

    /// Decode an item `arg` produced by [`Action::to_arg`].
    pub fn parse(arg: &str) -> Result<Action> {
        let arg = arg.trim();
        let (verb, payload) = arg.split_once(' ').unwrap_or((arg, ""));
        let payload = payload.trim();

        let action = match verb {
            "window" => Action::Window {
                url: non_empty(payload),
            },
            "windows" => Action::Windows {
                count: payload
                    .parse()
                    .map_err(|_| anyhow!("invalid window count: {payload:?}"))?,
            },
            "private" => Action::Private {
                url: non_empty(payload),
            },
            "tab" => Action::Tab {
                url: required(payload, "url")?,
            },
            "profile" => Action::Profile {
                dir: required(payload, "profile")?,
            },
            "profile-tab" => Action::ProfileWithCurrentTab {
                dir: required(payload, "profile")?,
            },
            "focus" => {
                let (window_id, tab_id) = parse_tab_ref(payload)?;

                Action::Focus { window_id, tab_id }
            }
            "copy" => Action::Copy {
                url: required(payload, "url")?,
            },
            "copy-md" => {
                let (url, title) = payload.split_once(' ').unwrap_or((payload, ""));

                Action::CopyMarkdown {
                    url: required(url, "url")?,
                    title: title.trim().to_owned(),
                }
            }
            "edit" => Action::Edit {
                url: required(payload, "url")?,
            },
            "close-url" => Action::CloseUrl {
                url: required(payload, "url")?,
            },
            "open" => {
                let (url, dir) = payload
                    .split_once(' ')
                    .ok_or_else(|| anyhow!("missing profile in {arg:?}"))?;

                Action::OpenInProfile {
                    url: required(url, "url")?,
                    dir: required(dir.trim(), "profile")?,
                }
            }
            "move-private" => {
                let (tab_ref, url) = payload
                    .split_once(' ')
                    .ok_or_else(|| anyhow!("missing url in {arg:?}"))?;
                let (window_id, tab_id) = parse_tab_ref(tab_ref)?;

                Action::MoveToPrivate {
                    window_id,
                    tab_id,
                    url: required(url.trim(), "url")?,
                }
            }
            other => bail!("unknown action: {other:?}"),
        };

        Ok(action)
    }
}

fn join(verb: &str, fields: &[&str]) -> String {
    let mut arg = verb.to_owned();

    for field in fields {
        if !field.is_empty() {
            arg.push(' ');
            arg.push_str(field);
        }
    }

    arg
}

fn non_empty(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }

    Some(value.to_owned())
}

fn required(value: &str, what: &str) -> Result<String> {
    non_empty(value).ok_or_else(|| anyhow!("missing {what}"))
}

fn parse_tab_ref(value: &str) -> Result<(String, String)> {
    let (window_id, tab_id) = value
        .split_once(':')
        .ok_or_else(|| anyhow!("invalid tab reference: {value:?}"))?;

    Ok((
        required(window_id, "window id")?,
        required(tab_id, "tab id")?,
    ))
}

/// Put `text` on the clipboard.
pub fn copy_to_clipboard(runner: &dyn CommandRunner, text: &str) -> Result<()> {
    runner
        .run(brave::OSASCRIPT, &["-e", CLIPBOARD_SCRIPT, text])
        .map(|_| ())
}

/// Show Alfred with `query` typed in.
pub fn alfred_search(runner: &dyn CommandRunner, query: &str) -> Result<()> {
    runner
        .run(brave::OSASCRIPT, &["-e", ALFRED_SEARCH_SCRIPT, query])
        .map(|_| ())
}

/// Perform `action`. Returns the text of the notification to show, if any.
pub fn execute(runner: &dyn CommandRunner, action: &Action) -> Result<Option<String>> {
    match action {
        Action::Window { url } => {
            brave::new_window(runner, WindowMode::Normal, url.as_deref(), 1)?;

            Ok(None)
        }
        Action::Windows { count } => {
            brave::new_window(runner, WindowMode::Normal, None, *count)?;

            Ok(None)
        }
        Action::Private { url } => {
            brave::new_window(runner, WindowMode::Private, url.as_deref(), 1)?;

            Ok(None)
        }
        Action::Tab { url } => {
            brave::new_tab(runner, url)?;

            Ok(None)
        }
        Action::Profile { dir } => {
            brave::open_in_profile(runner, dir, None)?;

            Ok(None)
        }
        Action::ProfileWithCurrentTab { dir } => {
            let tab = brave::current_tab(runner)?;
            let url = tab.as_ref().map(|tab| tab.url.as_str());

            brave::open_in_profile(runner, dir, url)?;

            Ok(None)
        }
        Action::Focus { window_id, tab_id } => {
            brave::focus_tab(runner, window_id, tab_id)?;

            Ok(None)
        }
        Action::Copy { url } => {
            copy_to_clipboard(runner, url)?;

            Ok(Some(format!("Copied {url}")))
        }
        Action::CopyMarkdown { url, title } => {
            let link = markdown::link(title, url);
            copy_to_clipboard(runner, &link)?;

            Ok(Some(format!("Copied {link}")))
        }
        Action::Edit { url } => {
            alfred_search(runner, &format!("{WINDOW_KEYWORD} {url}"))?;

            Ok(None)
        }
        Action::CloseUrl { url } => {
            let closed = brave::close_tabs_with_url(runner, url)?;
            let plural = if closed == 1 { "" } else { "s" };

            Ok(Some(format!("Closed {closed} tab{plural}")))
        }
        Action::OpenInProfile { url, dir } => {
            brave::open_in_profile(runner, dir, Some(url))?;

            Ok(None)
        }
        Action::MoveToPrivate {
            window_id,
            tab_id,
            url,
        } => {
            brave::new_window(runner, WindowMode::Private, Some(url), 1)?;
            brave::close_tab(runner, window_id, tab_id)?;

            Ok(None)
        }
    }
}
