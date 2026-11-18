//! Tests for the script filter items.
//!
//! `powerpack::Item`'s fields are private, so we assert on its JSON shape — the
//! same JSON Alfred consumes.

use alfred_brave::action::Action;
use alfred_brave::alfred;
use alfred_brave::history::Entry;
use alfred_brave::profiles::Profile;
use alfred_brave::query::WindowRequest;
use alfred_brave::tabs::Tab;

fn to_json(item: &powerpack::Item) -> serde_json::Value {
    serde_json::to_value(item).expect("item should serialize")
}

fn tab() -> Tab {
    Tab {
        window_id: "10".to_owned(),
        tab_id: "11".to_owned(),
        title: "Rust docs".to_owned(),
        url: "https://doc.rust-lang.org/".to_owned(),
        private: true,
        active: true,
    }
}

#[test]
fn blank_window_item_offers_private_and_tab_modifiers() {
    let items = alfred::window_items(&WindowRequest::Blank);
    let item = to_json(&items[0]);

    assert_eq!(item["arg"], "window");
    assert_eq!(item["mods"]["cmd"]["arg"], "private");
    assert_eq!(item["mods"]["alt"]["arg"], "tab about:blank");
}

#[test]
fn search_item_names_the_query_and_opens_its_url() {
    let items = alfred::window_items(&WindowRequest::Search {
        query: "rust".to_owned(),
        url: "https://search.brave.com/search?q=rust".to_owned(),
    });
    let item = to_json(&items[0]);

    assert_eq!(item["title"], "Search Brave for “rust”");
    // A URL or search lands in the current window; ⌥ opens a new one.
    assert_eq!(item["arg"], "tab https://search.brave.com/search?q=rust");
    assert_eq!(
        item["mods"]["alt"]["arg"],
        "window https://search.brave.com/search?q=rust"
    );
    assert_eq!(
        item["mods"]["cmd"]["arg"],
        "private https://search.brave.com/search?q=rust"
    );
}

#[test]
fn url_item_opens_in_the_current_window_by_default() {
    let item = to_json(&alfred::window_items(&WindowRequest::Url("https://a.test".to_owned()))[0]);

    assert_eq!(item["title"], "Open https://a.test");
    assert_eq!(item["arg"], "tab https://a.test");
    assert_eq!(item["mods"]["alt"]["arg"], "window https://a.test");
}

#[test]
fn window_count_item_has_no_modifier() {
    let item = to_json(&alfred::window_items(&WindowRequest::Count(3))[0]);

    assert_eq!(item["arg"], "windows 3");
    assert!(item.get("mods").is_none());
}

#[test]
fn tab_items_focus_on_enter_and_expose_every_modifier() {
    let item = to_json(&alfred::tab_items(vec![tab()])[0]);

    assert_eq!(item["title"], "Rust docs");
    assert_eq!(
        item["subtitle"],
        "Current tab · Private · https://doc.rust-lang.org/"
    );
    assert_eq!(item["arg"], "focus 10:11");
    assert_eq!(
        item["mods"]["cmd"]["arg"],
        "copy https://doc.rust-lang.org/"
    );
    assert_eq!(
        item["mods"]["cmd+shift"]["arg"],
        "copy-md https://doc.rust-lang.org/ Rust docs"
    );
    assert_eq!(
        item["mods"]["alt"]["arg"],
        "edit https://doc.rust-lang.org/"
    );
    assert_eq!(
        item["mods"]["ctrl"]["arg"],
        "close-url https://doc.rust-lang.org/"
    );
    // No uid: Alfred must keep our order.
    assert!(item.get("uid").is_none());
}

#[test]
fn every_modifier_arg_decodes_to_an_action() {
    let items = alfred::tab_items(vec![tab()]);
    let item = to_json(&items[0]);

    for (_, modifier) in item["mods"].as_object().unwrap() {
        let arg = modifier["arg"].as_str().unwrap();

        assert!(Action::parse(arg).is_ok(), "undecodable arg {arg:?}");
    }
}

#[test]
fn empty_lists_yield_a_single_invalid_item() {
    for items in [
        alfred::tab_items(Vec::new()),
        alfred::profile_items(Vec::new()),
        alfred::history_items(Vec::new(), true),
    ] {
        assert_eq!(items.len(), 1);
        let item = to_json(&items[0]);
        assert_eq!(item["valid"], false);
        assert!(item.get("arg").is_none());
    }
}

#[test]
fn profile_items_open_the_profile_directory() {
    let item = to_json(
        &alfred::profile_items(vec![Profile {
            dir: "Profile 1".to_owned(),
            name: "Work".to_owned(),
            active_time: 0.0,
        }])[0],
    );

    assert_eq!(item["title"], "Work");
    assert_eq!(item["arg"], "profile Profile 1");
    assert_eq!(item["mods"]["cmd"]["arg"], "profile-tab Profile 1");
}

#[test]
fn history_items_mention_the_profile_only_when_there_are_several() {
    let entry = Entry {
        url: "https://a.test".to_owned(),
        title: "A".to_owned(),
        last_visit: 0,
        profile_dir: "Profile 1".to_owned(),
        profile_name: "Work".to_owned(),
    };

    let several = to_json(&alfred::history_items(vec![entry.clone()], true)[0]);
    assert_eq!(several["subtitle"], "Work · https://a.test");
    assert_eq!(several["arg"], "open https://a.test Profile 1");

    let single = to_json(&alfred::history_items(vec![entry], false)[0]);
    assert_eq!(single["subtitle"], "https://a.test");
}

#[test]
fn current_tab_items_need_a_tab() {
    let missing = to_json(&alfred::copy_current_items(None)[0]);
    assert_eq!(missing["title"], "Brave is not running");

    let copy = to_json(&alfred::copy_current_items(Some(tab()))[0]);
    assert_eq!(copy["arg"], "copy https://doc.rust-lang.org/");

    let private = to_json(&alfred::private_current_items(Some(tab()))[0]);
    assert_eq!(private["arg"], "private https://doc.rust-lang.org/");
    assert_eq!(
        private["mods"]["cmd"]["arg"],
        "move-private 10:11 https://doc.rust-lang.org/"
    );
}
