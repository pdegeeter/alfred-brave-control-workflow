//! Script filter items for every keyword.
//!
//! Every actionable item (and each of its modifiers) carries an
//! [`Action`] encoded in its `arg`; the one connected Run Script decodes and
//! performs it. No `uid` is set: Alfred would otherwise re-sort items by its
//! own usage knowledge and override the order computed here (front window
//! first, most recent profile or visit first).

use powerpack::{Item, Key, Modifier};

use crate::action::Action;
use crate::history::Entry;
use crate::profiles::Profile;
use crate::query::WindowRequest;
use crate::tabs::Tab;

/// Item shown in place of a list when something went wrong.
pub fn error_item(title: &str, message: &str) -> Item {
    Item::new(title).subtitle(message).valid(false)
}

/// Item shown when Brave is not running.
pub fn not_running_item() -> Item {
    error_item("Brave is not running", "Start Brave Browser first")
}

fn modifier(key: Key, subtitle: &str, action: &Action) -> Modifier {
    Modifier::new(key).subtitle(subtitle).arg(action.to_arg())
}

fn copy_modifiers(item: Item, url: &str, title: &str) -> Item {
    item.modifier(modifier(
        Key::Command,
        "Copy URL",
        &Action::Copy {
            url: url.to_owned(),
        },
    ))
    .modifier(
        Modifier::new_multi([Key::Command, Key::Shift])
            .subtitle("Copy as Markdown link")
            .arg(
                Action::CopyMarkdown {
                    url: url.to_owned(),
                    title: title.to_owned(),
                }
                .to_arg(),
            ),
    )
}

/// `bw`: the single item describing what <kbd>Enter</kbd> will open.
pub fn window_items(request: &WindowRequest) -> Vec<Item> {
    let item = match request {
        WindowRequest::Blank => Item::new("New Brave window")
            .subtitle("Open a window in the current space · ⌘ private · ⌥ new tab")
            .arg(Action::Window { url: None }.to_arg())
            .modifier(modifier(
                Key::Command,
                "New private window",
                &Action::Private { url: None },
            ))
            .modifier(modifier(
                Key::Option,
                "New tab in the front window",
                &Action::Tab {
                    url: "about:blank".to_owned(),
                },
            )),
        WindowRequest::Count(count) => Item::new(format!("Open {count} Brave windows"))
            .subtitle("In the current space")
            .arg(Action::Windows { count: *count }.to_arg()),
        WindowRequest::Url(url) => url_item(format!("Open {url}"), url),
        WindowRequest::Search { query, url } => {
            url_item(format!("Search Brave for “{query}”"), url)
        }
    };

    vec![item]
}

/// Item opening `url` in a new window, a private window (⌘) or a tab (⌥).
fn url_item(title: String, url: &str) -> Item {
    Item::new(title)
        .subtitle("In a new window · ⌘ private window · ⌥ new tab")
        .arg(
            Action::Window {
                url: Some(url.to_owned()),
            }
            .to_arg(),
        )
        .modifier(modifier(
            Key::Command,
            "Open in a new private window",
            &Action::Private {
                url: Some(url.to_owned()),
            },
        ))
        .modifier(modifier(
            Key::Option,
            "Open in a new tab of the front window",
            &Action::Tab {
                url: url.to_owned(),
            },
        ))
}

/// `bwp`: one item per profile.
pub fn profile_items(profiles: Vec<Profile>) -> Vec<Item> {
    if profiles.is_empty() {
        return vec![error_item(
            "No matching Brave profile",
            "Profiles are read from Brave's Local State file",
        )];
    }

    profiles
        .into_iter()
        .map(|profile| {
            Item::new(profile.name.clone())
                .subtitle(format!(
                    "Open a new window in this profile ({})",
                    profile.dir
                ))
                .arg(
                    Action::Profile {
                        dir: profile.dir.clone(),
                    }
                    .to_arg(),
                )
                .autocomplete(profile.name.clone())
                .modifier(modifier(
                    Key::Command,
                    "Open the current tab in this profile",
                    &Action::ProfileWithCurrentTab { dir: profile.dir },
                ))
        })
        .collect()
}

/// `bwt`: one item per open tab.
pub fn tab_items(tabs: Vec<Tab>) -> Vec<Item> {
    if tabs.is_empty() {
        return vec![error_item("No matching Brave tab", "Try another search")];
    }

    tabs.into_iter()
        .map(|tab| {
            let mut subtitle = tab.url.clone();

            if tab.private {
                subtitle = format!("Private · {subtitle}");
            }

            if tab.active {
                subtitle = format!("Current tab · {subtitle}");
            }

            let item = Item::new(tab.display_title())
                .subtitle(subtitle)
                .arg(
                    Action::Focus {
                        window_id: tab.window_id.clone(),
                        tab_id: tab.tab_id.clone(),
                    }
                    .to_arg(),
                )
                .copy_text(tab.url.clone())
                .modifier(modifier(
                    Key::Option,
                    "Edit the URL",
                    &Action::Edit {
                        url: tab.url.clone(),
                    },
                ))
                .modifier(modifier(
                    Key::Control,
                    "Close every tab with this URL",
                    &Action::CloseUrl {
                        url: tab.url.clone(),
                    },
                ));

            copy_modifiers(item, &tab.url, &tab.title)
        })
        .collect()
}

/// `bhi`: one item per history entry.
pub fn history_items(entries: Vec<Entry>, show_profile: bool) -> Vec<Item> {
    if entries.is_empty() {
        return vec![error_item(
            "No matching history entry",
            "Try another search",
        )];
    }

    entries
        .into_iter()
        .map(|entry| {
            let subtitle = if show_profile {
                format!("{} · {}", entry.profile_name, entry.url)
            } else {
                entry.url.clone()
            };

            let item = Item::new(entry.display_title())
                .subtitle(subtitle)
                .arg(
                    Action::OpenInProfile {
                        url: entry.url.clone(),
                        dir: entry.profile_dir.clone(),
                    }
                    .to_arg(),
                )
                .copy_text(entry.url.clone())
                .modifier(modifier(
                    Key::Option,
                    "Edit the URL",
                    &Action::Edit {
                        url: entry.url.clone(),
                    },
                ));

            copy_modifiers(item, &entry.url, &entry.title)
        })
        .collect()
}

/// `bwc`: copy the current tab's URL.
pub fn copy_current_items(tab: Option<Tab>) -> Vec<Item> {
    let Some(tab) = tab else {
        return vec![not_running_item()];
    };

    let item = Item::new(format!("Copy URL of “{}”", tab.display_title()))
        .subtitle(format!("{} · ⇧⌘ as Markdown", tab.url))
        .arg(
            Action::Copy {
                url: tab.url.clone(),
            }
            .to_arg(),
        )
        .modifier(
            Modifier::new_multi([Key::Command, Key::Shift])
                .subtitle("Copy as Markdown link")
                .arg(
                    Action::CopyMarkdown {
                        url: tab.url.clone(),
                        title: tab.title.clone(),
                    }
                    .to_arg(),
                ),
        );

    vec![item]
}

/// `btp`: reopen the current tab in a private window.
pub fn private_current_items(tab: Option<Tab>) -> Vec<Item> {
    let Some(tab) = tab else {
        return vec![not_running_item()];
    };

    let item = Item::new(format!(
        "Open “{}” in a private window",
        tab.display_title()
    ))
    .subtitle(format!("{} · ⌘ also close the tab", tab.url))
    .arg(
        Action::Private {
            url: Some(tab.url.clone()),
        }
        .to_arg(),
    )
    .modifier(modifier(
        Key::Command,
        "Open in a private window and close this tab",
        &Action::MoveToPrivate {
            window_id: tab.window_id.clone(),
            tab_id: tab.tab_id.clone(),
            url: tab.url.clone(),
        },
    ));

    vec![item]
}
