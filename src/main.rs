//! `alfred-brave` — Alfred workflow entry point.
//!
//! Subcommands, dispatched by Alfred:
//!   * `window [query]`   — script filter `bw`: new window(s), URL or search.
//!   * `profiles [query]` — script filter `bwp`: Brave profiles.
//!   * `tabs [query]`     — script filter `bwt`: open tabs.
//!   * `history [query]`  — script filter `bhi`: history of every profile.
//!   * `current`          — script filter `bwc`: copy the current tab URL.
//!   * `private-tab`      — script filter `btp`: current tab → private window.
//!   * `run <arg>`        — Run Script action: perform the selected item.

use std::env;
use std::process::ExitCode;

use anyhow::{Context, Result};
use powerpack::Item;

use alfred_brave::action::{self, Action};
use alfred_brave::runner::{CommandRunner, SystemRunner};
use alfred_brave::{alfred, brave, history, profiles, query, tabs};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_default();
    let arg = args.next().unwrap_or_default();
    let runner = SystemRunner;

    match command.as_str() {
        "window" => output(Ok(alfred::window_items(&query::parse_window_request(&arg)))),
        "profiles" => output(Ok(list_profiles(&arg))),
        "tabs" => output(list_tabs(&runner, &arg)),
        "history" => output(list_history(&runner, &arg)),
        "current" => output(brave::current_tab(&runner).map(alfred::copy_current_items)),
        "private-tab" => output(brave::current_tab(&runner).map(alfred::private_current_items)),
        "run" => run(&runner, &arg),
        other => {
            eprintln!(
                "unknown command: `{other}` (expected window, profiles, tabs, history, current, private-tab or run)"
            );

            ExitCode::FAILURE
        }
    }
}

/// Print script filter items. A failure becomes an Alfred item rather than a
/// crash, so the user sees a readable message in the dropdown.
fn output(items: Result<Vec<Item>>) -> ExitCode {
    let items =
        items.unwrap_or_else(|err| vec![alfred::error_item("Brave error", &format!("{err:#}"))]);

    match powerpack::output(items) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("failed to write Alfred output: {err}");

            ExitCode::FAILURE
        }
    }
}

fn list_profiles(query: &str) -> Vec<Item> {
    let all = profiles::default_user_data_dir()
        .map(|dir| profiles::load(&dir))
        .unwrap_or_default();

    alfred::profile_items(profiles::filter(all, query))
}

fn list_tabs(runner: &dyn CommandRunner, query: &str) -> Result<Vec<Item>> {
    let Some(all) = brave::list_tabs(runner)? else {
        return Ok(vec![alfred::not_running_item()]);
    };

    Ok(alfred::tab_items(tabs::filter(all, query)))
}

fn list_history(runner: &dyn CommandRunner, query: &str) -> Result<Vec<Item>> {
    let user_data_dir = profiles::default_user_data_dir().context("HOME is not set")?;
    let all = profiles::load(&user_data_dir);
    let entries = history::search(runner, &user_data_dir, &all, query)?;

    Ok(alfred::history_items(entries, all.len() > 1))
}

/// Run Script action: decode the item's `arg` and perform it. Whatever is
/// printed becomes the text of the connected notification.
fn run(runner: &dyn CommandRunner, arg: &str) -> ExitCode {
    let result = Action::parse(arg).and_then(|parsed| action::execute(runner, &parsed));

    match result {
        Ok(message) => {
            if let Some(message) = message {
                println!("{message}");
            }

            ExitCode::SUCCESS
        }
        Err(err) => {
            println!("Brave: {err}");
            eprintln!("{err:#}");

            ExitCode::FAILURE
        }
    }
}
