//! Everything that drives Brave Browser itself.
//!
//! Brave exposes Chromium's AppleScript dictionary (`windows`, `tabs`,
//! `activeTabIndex`, window `mode`), driven here through JavaScript for
//! Automation (`osascript -l JavaScript`). Each script is a fixed constant that
//! takes its inputs as `argv`, so nothing user-controlled is ever interpolated
//! into script source. Opening a window in a given profile is not scriptable,
//! so it goes through Brave's command line (`--profile-directory`) via macOS
//! `open`.
//!
//! The listing scripts never launch Brave: they return `null` when it is not
//! running, which [`list_tabs`] and [`current_tab`] map to `None`.

use anyhow::{Context, Result};

use crate::runner::CommandRunner;
use crate::tabs::Tab;

/// Application name used by `open -a` and JXA's `Application(...)`.
pub const APP_NAME: &str = "Brave Browser";

/// Path of the `osascript` binary.
pub const OSASCRIPT: &str = "/usr/bin/osascript";

/// Path of macOS `open`.
pub const OPEN: &str = "/usr/bin/open";

/// Every tab of every window, front window first, or `null` when Brave is not
/// running.
const LIST_TABS_SCRIPT: &str = r#"
function run() {
  const brave = Application("Brave Browser");
  if (!brave.running()) {
    return "null";
  }

  const windows = brave.windows;
  const windowIds = windows.id();
  const modes = windows.mode();
  const activeIndexes = windows.activeTabIndex();
  const tabIds = windows.tabs.id();
  const titles = windows.tabs.title();
  const urls = windows.tabs.url();
  const tabs = [];

  for (let w = 0; w < windowIds.length; w++) {
    for (let t = 0; t < tabIds[w].length; t++) {
      tabs.push({
        windowId: String(windowIds[w]),
        tabId: String(tabIds[w][t]),
        title: titles[w][t] || "",
        url: urls[w][t] || "",
        private: modes[w] === "incognito",
        active: activeIndexes[w] === t + 1 && w === 0,
      });
    }
  }

  return JSON.stringify(tabs);
}
"#;

/// The active tab of the front window, or `null` when Brave is not running or
/// has no window.
const CURRENT_TAB_SCRIPT: &str = r#"
function run() {
  const brave = Application("Brave Browser");
  if (!brave.running() || brave.windows.length === 0) {
    return "null";
  }

  const win = brave.windows[0];
  const tab = win.activeTab;

  return JSON.stringify({
    windowId: String(win.id()),
    tabId: String(tab.id()),
    title: tab.title() || "",
    url: tab.url() || "",
    private: win.mode() === "incognito",
    active: true,
  });
}
"#;

/// argv: window id, tab id. Selects the tab and raises its window.
const FOCUS_TAB_SCRIPT: &str = r#"
function run(argv) {
  const brave = Application("Brave Browser");
  const win = brave.windows.byId(Number(argv[0]));
  const index = win.tabs.id().map(String).indexOf(argv[1]);
  if (index < 0) {
    throw new Error("tab " + argv[1] + " not found");
  }

  win.activeTabIndex = index + 1;
  win.index = 1;
  brave.activate();

  return "";
}
"#;

/// argv: window id, tab id. Closes that tab.
const CLOSE_TAB_SCRIPT: &str = r#"
function run(argv) {
  const brave = Application("Brave Browser");
  brave.windows.byId(Number(argv[0])).tabs.byId(Number(argv[1])).close();

  return "";
}
"#;

/// argv: url. Closes every tab showing exactly that URL; prints the count.
const CLOSE_URL_SCRIPT: &str = r#"
function run(argv) {
  const brave = Application("Brave Browser");
  if (!brave.running()) {
    return "0";
  }

  let closed = 0;
  const windows = brave.windows();
  for (let w = windows.length - 1; w >= 0; w--) {
    const tabs = windows[w].tabs();
    for (let t = tabs.length - 1; t >= 0; t--) {
      if (tabs[t].url() === argv[0]) {
        tabs[t].close();
        closed++;
      }
    }
  }

  return String(closed);
}
"#;

/// argv: mode (`normal` | `incognito`), url (may be empty), count. Opens
/// `count` windows in the current space.
const NEW_WINDOW_SCRIPT: &str = r#"
function run(argv) {
  const brave = Application("Brave Browser");
  const mode = argv[0];
  const url = argv[1];
  const count = Number(argv[2]);

  for (let i = 0; i < count; i++) {
    brave.Window({ mode: mode }).make();
    if (url) {
      brave.windows[0].activeTab.url = url;
    }
  }
  brave.activate();

  return "";
}
"#;

/// argv: url. Opens a tab in the front window, or a window if there is none.
const NEW_TAB_SCRIPT: &str = r#"
function run(argv) {
  const brave = Application("Brave Browser");
  if (brave.windows.length === 0) {
    brave.Window().make();
    brave.windows[0].activeTab.url = argv[0];
  } else {
    const win = brave.windows[0];
    win.tabs.push(brave.Tab({ url: argv[0] }));
    win.activeTabIndex = win.tabs.length;
  }
  brave.activate();

  return "";
}
"#;

/// Window flavour for [`new_window`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowMode {
    Normal,
    Private,
}

impl WindowMode {
    fn as_jxa(self) -> &'static str {
        match self {
            WindowMode::Normal => "normal",
            WindowMode::Private => "incognito",
        }
    }
}

fn run_jxa(runner: &dyn CommandRunner, script: &str, args: &[&str]) -> Result<String> {
    let mut full_args = vec!["-l", "JavaScript", "-e", script];
    full_args.extend_from_slice(args);

    runner.run(OSASCRIPT, &full_args)
}

/// Parse the JSON printed by the listing scripts. `null` means Brave is not
/// running.
pub fn parse_tabs(json: &str) -> Result<Option<Vec<Tab>>> {
    serde_json::from_str(json.trim()).context("unexpected tab list from Brave")
}

/// Parse the JSON printed by the current-tab script. `null` means Brave is not
/// running or has no window.
pub fn parse_current_tab(json: &str) -> Result<Option<Tab>> {
    serde_json::from_str(json.trim()).context("unexpected current tab from Brave")
}

/// Every open tab, front window first; `None` when Brave is not running.
pub fn list_tabs(runner: &dyn CommandRunner) -> Result<Option<Vec<Tab>>> {
    parse_tabs(&run_jxa(runner, LIST_TABS_SCRIPT, &[])?)
}

/// The active tab of the front window; `None` when there is none.
pub fn current_tab(runner: &dyn CommandRunner) -> Result<Option<Tab>> {
    parse_current_tab(&run_jxa(runner, CURRENT_TAB_SCRIPT, &[])?)
}

/// Select a tab and bring its window to the front.
pub fn focus_tab(runner: &dyn CommandRunner, window_id: &str, tab_id: &str) -> Result<()> {
    run_jxa(runner, FOCUS_TAB_SCRIPT, &[window_id, tab_id]).map(|_| ())
}

/// Close one tab.
pub fn close_tab(runner: &dyn CommandRunner, window_id: &str, tab_id: &str) -> Result<()> {
    run_jxa(runner, CLOSE_TAB_SCRIPT, &[window_id, tab_id]).map(|_| ())
}

/// Close every tab showing exactly `url`; returns how many were closed.
pub fn close_tabs_with_url(runner: &dyn CommandRunner, url: &str) -> Result<usize> {
    let output = run_jxa(runner, CLOSE_URL_SCRIPT, &[url])?;

    output
        .trim()
        .parse()
        .with_context(|| format!("unexpected closed-tab count from Brave: {output:?}"))
}

/// Open `count` new windows (each on `url` when given) in the current space.
pub fn new_window(
    runner: &dyn CommandRunner,
    mode: WindowMode,
    url: Option<&str>,
    count: u32,
) -> Result<()> {
    let count = count.to_string();

    run_jxa(
        runner,
        NEW_WINDOW_SCRIPT,
        &[mode.as_jxa(), url.unwrap_or(""), &count],
    )
    .map(|_| ())
}

/// Open `url` in a new tab of the front window.
pub fn new_tab(runner: &dyn CommandRunner, url: &str) -> Result<()> {
    run_jxa(runner, NEW_TAB_SCRIPT, &[url]).map(|_| ())
}

/// Open something in the profile stored in `profile_dir` (e.g. `Default`,
/// `Profile 1`):
///
/// - with a `url`, in a new tab of that profile's current window (Brave only
///   creates a window when the profile has none);
/// - without one, in a new blank window of that profile.
pub fn open_in_profile(
    runner: &dyn CommandRunner,
    profile_dir: &str,
    url: Option<&str>,
) -> Result<()> {
    let profile_flag = format!("--profile-directory={profile_dir}");
    let mut args = vec!["-na", APP_NAME, "--args", &profile_flag];

    match url {
        Some(url) => args.push(url),
        None => args.push("--new-window"),
    }

    runner.run(OPEN, &args).map(|_| ())
}
