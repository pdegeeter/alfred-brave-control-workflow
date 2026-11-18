# Alfred Brave Control Workflow

An [Alfred](https://www.alfredapp.com/) workflow, written in Rust, to control
[Brave Browser](https://brave.com/) from Alfred: open windows in the current
space, jump into private browsing, switch between profiles, search your
history and move through tabs.

```
bwt rust
 ┌─────────────────────────────────────────────────────────┐
 │ The Rust Programming Language                             │
 │   Current tab · https://doc.rust-lang.org/book/           │
 │ rust-lang/rust: Empowering everyone to build reliable…    │
 │   https://github.com/rust-lang/rust                       │
 └─────────────────────────────────────────────────────────┘
```

## Usage

### New window — `bw`

Opens a new Brave window in the current space.

| Command            | Description                                                  |
| ------------------ | ------------------------------------------------------------ |
| `bw`               | Open a new window.                                           |
| `bw x3`            | Open 3 new windows (at most 20).                             |
| `bw github.com`    | Open a URL in a new window (`https://` is added to bare hosts, `http://` to `localhost`). |
| `bw rust lifetimes`| Search Brave Search in a new window.                         |

- <kbd>⌘</kbd><kbd>↩</kbd> open in a new **private** window
- <kbd>⌥</kbd><kbd>↩</kbd> open in a new **tab** of the front window

### Profiles — `bwp`

Lists your Brave profiles, most recently used first. They are read from
Brave's own `Local State` file, so there is nothing to configure.

- <kbd>↩</kbd> open a new window in that profile
- <kbd>⌘</kbd><kbd>↩</kbd> open the current tab in that profile

### Tabs — `bwt`

Lists every open tab (front window first); search by title or URL.

- <kbd>↩</kbd> switch to the tab
- <kbd>⌘</kbd><kbd>↩</kbd> copy its URL
- <kbd>⇧</kbd><kbd>⌘</kbd><kbd>↩</kbd> copy it as a Markdown link
- <kbd>⌥</kbd><kbd>↩</kbd> edit the URL (reopens Alfred on `bw <url>`)
- <kbd>⌃</kbd><kbd>↩</kbd> close every tab showing this URL

### History — `bhi`

Searches the browsing history of **every** profile by title or URL, most
recent visit first. The profile is shown when you have more than one.

- <kbd>↩</kbd> open the page in a new window of the profile it comes from
- <kbd>⌘</kbd><kbd>↩</kbd> copy its URL
- <kbd>⇧</kbd><kbd>⌘</kbd><kbd>↩</kbd> copy it as a Markdown link
- <kbd>⌥</kbd><kbd>↩</kbd> edit the URL (reopens Alfred on `bw <url>`)

### Copy current URL — `bwc`

Copies the URL of the current tab.

- <kbd>⇧</kbd><kbd>⌘</kbd><kbd>↩</kbd> copy it as a Markdown link

### Current tab in private — `btp`

Reopens the current tab in a new private window.

- <kbd>⌘</kbd><kbd>↩</kbd> also close the original tab

Configure [Hotkeys](https://www.alfredapp.com/help/workflows/triggers/hotkey/)
in front of any keyword for faster triggering.

## How it works

- **Windows and tabs** are driven through Brave's AppleScript dictionary (the
  Chromium one) with JavaScript for Automation (`osascript -l JavaScript`).
  Listing commands never launch Brave: when it is not running they say so.
- **Profiles** come from `profile.info_cache` in
  `~/Library/Application Support/BraveSoftware/Brave-Browser/Local State`.
  Opening a window in a profile uses Brave's command line
  (`open -na "Brave Browser" --args --profile-directory=<dir> --new-window`).
- **History** is read from each profile's `History` SQLite database with the
  system `sqlite3`, opened read-only and `immutable`, so it works while Brave
  is running and never touches Brave's lock. Search terms are matched in SQL,
  so your whole history is searchable, not only the latest entries.

## Requirements

- macOS (Apple Silicon / arm64)
- [Alfred 5](https://www.alfredapp.com/) with the **Powerpack** license
  (workflows are a paid feature)
- [Brave Browser](https://brave.com/)

The first time a command talks to Brave, macOS asks you to let Alfred control
Brave Browser (*System Settings → Privacy & Security → Automation*).

## Install

Download the latest `alfred-brave.alfredworkflow` from the
[Releases](https://github.com/pdegeeter/alfred-brave-control-workflow/releases)
page and double-click it to import into Alfred.

Or build and deploy from source:

```sh
cargo install powerpack-cli   # one-time
./scripts/deploy.sh           # build + symlink into Alfred
```

## Documentation

- [Architecture](docs/architecture.md) — components, data flow, design
  decisions.
- [Development](docs/development.md) — build, test, lint, deploy locally.
- [Release process](docs/release.md) — release-please and the CI/CD pipeline.

## Credits

Inspired by [Safari Control](https://github.com/vanstrouble/safari-control-alfred-workflow)
by Pedro Vázquez. The workflow icon is Brave Browser's application icon,
converted to `workflow/icon.png`; Brave is a trademark of Brave Software, Inc.

## License

MIT
