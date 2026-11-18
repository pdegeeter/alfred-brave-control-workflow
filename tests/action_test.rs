//! Tests for the `arg` encoding of actions and for executing them.

mod common;

use alfred_brave::action::{self, Action};
use alfred_brave::brave::{OPEN, OSASCRIPT};
use alfred_brave::markdown;
use common::FakeRunner;

fn all_actions() -> Vec<Action> {
    vec![
        Action::Window { url: None },
        Action::Window {
            url: Some("https://a.test".to_owned()),
        },
        Action::Windows { count: 3 },
        Action::Private { url: None },
        Action::Private {
            url: Some("https://a.test".to_owned()),
        },
        Action::Tab {
            url: "https://a.test".to_owned(),
        },
        Action::Profile {
            dir: "Profile 1".to_owned(),
        },
        Action::ProfileWithCurrentTab {
            dir: "Default".to_owned(),
        },
        Action::Focus {
            window_id: "10".to_owned(),
            tab_id: "11".to_owned(),
        },
        Action::Copy {
            url: "https://a.test".to_owned(),
        },
        Action::CopyMarkdown {
            url: "https://a.test".to_owned(),
            title: "A title with spaces".to_owned(),
        },
        Action::Edit {
            url: "https://a.test".to_owned(),
        },
        Action::CloseUrl {
            url: "https://a.test".to_owned(),
        },
        Action::OpenInProfile {
            url: "https://a.test".to_owned(),
            dir: "Profile 1".to_owned(),
        },
        Action::MoveToPrivate {
            window_id: "10".to_owned(),
            tab_id: "11".to_owned(),
            url: "https://a.test".to_owned(),
        },
    ]
}

#[test]
fn every_action_round_trips_through_its_arg() {
    for action in all_actions() {
        let arg = action.to_arg();

        assert_eq!(Action::parse(&arg).unwrap(), action, "arg: {arg:?}");
    }
}

#[test]
fn parse_rejects_unknown_or_incomplete_args() {
    assert!(Action::parse("explode now").is_err());
    assert!(Action::parse("tab").is_err());
    assert!(Action::parse("focus 10").is_err());
    assert!(Action::parse("windows many").is_err());
    assert!(Action::parse("open https://a.test").is_err());
}

#[test]
fn copy_puts_the_url_on_the_clipboard_and_reports_it() {
    let runner = FakeRunner::new().with_success(OSASCRIPT, "");

    let message = action::execute(
        &runner,
        &Action::Copy {
            url: "https://a.test".to_owned(),
        },
    )
    .unwrap();

    assert_eq!(message.as_deref(), Some("Copied https://a.test"));
    let call = &runner.calls()[0];
    assert_eq!(call.args[0], "-e");
    assert!(call.args[1].contains("set the clipboard"));
    assert_eq!(call.args[2], "https://a.test");
}

#[test]
fn copy_markdown_copies_a_link() {
    let runner = FakeRunner::new().with_success(OSASCRIPT, "");

    action::execute(
        &runner,
        &Action::CopyMarkdown {
            url: "https://a.test".to_owned(),
            title: "A [b]".to_owned(),
        },
    )
    .unwrap();

    assert_eq!(runner.calls()[0].args[2], "[A \\[b\\]](https://a.test)");
}

#[test]
fn edit_reopens_alfred_on_the_window_keyword() {
    let runner = FakeRunner::new().with_success(OSASCRIPT, "");

    action::execute(
        &runner,
        &Action::Edit {
            url: "https://a.test".to_owned(),
        },
    )
    .unwrap();

    let call = &runner.calls()[0];
    assert!(call.args[1].contains(action::ALFRED_BUNDLE_ID));
    assert_eq!(call.args[2], "bw https://a.test");
}

#[test]
fn close_url_reports_how_many_tabs_were_closed() {
    let runner = FakeRunner::new().with_success(OSASCRIPT, "1");

    let message = action::execute(
        &runner,
        &Action::CloseUrl {
            url: "https://a.test".to_owned(),
        },
    )
    .unwrap();

    assert_eq!(message.as_deref(), Some("Closed 1 tab"));
}

#[test]
fn profile_with_current_tab_opens_its_url_in_the_profile() {
    let runner = FakeRunner::new()
        .with_success(
            OSASCRIPT,
            r#"{"windowId":"1","tabId":"2","title":"A","url":"https://a.test","private":false,"active":true}"#,
        )
        .with_success(OPEN, "");

    action::execute(
        &runner,
        &Action::ProfileWithCurrentTab {
            dir: "Profile 1".to_owned(),
        },
    )
    .unwrap();

    assert_eq!(runner.programs_called(), [OSASCRIPT, OPEN]);
    let open = &runner.calls()[1];
    assert!(open
        .args
        .contains(&"--profile-directory=Profile 1".to_owned()));
    assert_eq!(open.args.last().unwrap(), "https://a.test");
}

#[test]
fn move_to_private_opens_the_private_window_before_closing_the_tab() {
    let runner = FakeRunner::new().with_success(OSASCRIPT, "");

    action::execute(
        &runner,
        &Action::MoveToPrivate {
            window_id: "10".to_owned(),
            tab_id: "11".to_owned(),
            url: "https://a.test".to_owned(),
        },
    )
    .unwrap();

    let calls = runner.calls();
    assert_eq!(
        FakeRunner::jxa_args(&calls[0]),
        ["incognito", "https://a.test", "1"]
    );
    assert_eq!(FakeRunner::jxa_args(&calls[1]), ["10", "11"]);
}

#[test]
fn move_to_private_keeps_the_tab_when_the_private_window_fails() {
    let runner = FakeRunner::new().with_failure(OSASCRIPT, "Brave refused");

    let result = action::execute(
        &runner,
        &Action::MoveToPrivate {
            window_id: "10".to_owned(),
            tab_id: "11".to_owned(),
            url: "https://a.test".to_owned(),
        },
    );

    assert!(result.is_err());
    assert_eq!(runner.calls().len(), 1);
}

#[test]
fn markdown_link_escapes_brackets_and_parentheses() {
    assert_eq!(
        markdown::link("a [b] c", "https://x.test/(1)"),
        "[a \\[b\\] c](https://x.test/%281%29)"
    );
    assert_eq!(
        markdown::link("  ", "https://x.test"),
        "[https://x.test](https://x.test)"
    );
}
