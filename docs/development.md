# Development

## Contributing

`main` is a protected branch:

- Changes land via pull requests — direct pushes are rejected (enforced for
  admins too).
- The CI checks `test` and `package` must be green before a PR can be merged.
- **All commits must be signed** (GitHub "require signed commits"). Configure
  SSH commit signing so your commits verify:

  ```sh
  git config gpg.format ssh
  git config user.signingkey ~/.ssh/<your-key>.pub
  git config commit.gpgsign true
  ```

  The signing key must be registered on your GitHub account as a *signing* key,
  and your commit email must be a verified GitHub email, otherwise the signature
  will not verify and the push/merge will be blocked.

## Prerequisites

- Rust (stable) — `rustup` recommended.
- [`powerpack-cli`](https://crates.io/crates/powerpack-cli): `cargo install powerpack-cli`.
- Brave Browser, for manual testing.
- Alfred 5 with Powerpack, to actually run the workflow.

## Common tasks

```sh
cargo test --all                          # run the test suite (offline, no Brave needed)
cargo clippy --all-targets -- -D warnings # lint (warnings are errors)
cargo fmt --all                           # format

./scripts/build.sh    # build release binary + package the .alfredworkflow
./scripts/deploy.sh   # build + update the installed workflow in place (imports it the first time)
./scripts/deploy.sh --import  # build + open the .alfredworkflow to import it
./scripts/icon.sh     # regenerate workflow/icon.png from Brave's app icon
```

The workflow icon (`workflow/icon.png`) is a committed asset converted from
`/Applications/Brave Browser.app/Contents/Resources/app.icns` with `sips`. It is
not regenerated at build time, so CI needs neither Brave nor image tooling.

`scripts/deploy.sh` finds the installed workflow in Alfred's preferences
(`current` in `~/Library/Application Support/Alfred/prefs.json`, or
`$ALFRED_PREFERENCES`) by its bundle id and copies the new binary,
`info.plist` and icon over it; Alfred reloads the folder. When the workflow is
not installed yet, it opens the `.alfredworkflow` for import instead. It does
not symlink (`powerpack link`): Alfred 6 turns the link into a stale copy, and
re-importing over an existing install fails. An old symlinked install must be
deleted in Alfred Preferences and imported once with `--import`.

`scripts/build.sh` syncs `workflow/info.plist`'s `<version>` from `Cargo.toml`
before packaging, so the version shown in Alfred always matches the crate.

## Manual smoke test (without Alfred)

The binary speaks Alfred's JSON on stdout, so you can exercise it directly:

```sh
cargo build
B=./target/debug/alfred-brave

$B window "github.com/rust-lang" | python3 -m json.tool   # bw
$B profiles | python3 -m json.tool                        # bwp
$B tabs rust | python3 -m json.tool                       # bwt
$B history "rust docs" | python3 -m json.tool             # bwh
$B current | python3 -m json.tool                         # bwc

# Actions take the `arg` of an item (see docs/architecture.md):
$B run "private https://example.com"
$B run "close-url https://example.com"
```

The first `osascript` call from a new process (Terminal, Alfred) triggers the
macOS Automation prompt for Brave Browser.

## Testing strategy

All logic is pure and unit-tested; every process spawn — `osascript`,
`sqlite3`, `open` — goes through the `CommandRunner` trait and is replaced in
tests by `FakeRunner` (`tests/common/mod.rs`), which returns queued canned
outputs per program and records every call. The whole suite runs offline,
without Brave.

| Test file                | Covers |
| ------------------------ | ------ |
| `tests/query_test.rs`    | `bw` query parsing: blank, `xN` (capped), URLs, bare hosts, `localhost`, Brave Search fallback, encoding. |
| `tests/profiles_test.rs` | `Local State` parsing, ordering by activity, name fallback, missing/malformed files, filtering. |
| `tests/tabs_test.rs`     | JXA output parsing (`null` = not running), tab filtering, the arguments passed to every Brave script and to `open`. |
| `tests/history_test.rs`  | SQL building and escaping, `immutable` URI, Chromium timestamps, row parsing, cross-profile merge, partial failures. |
| `tests/action_test.rs`   | `arg` round-trip of every action, invalid args, execution sequences (clipboard, Alfred search, move to private ordering). |
| `tests/alfred_test.rs`   | JSON shape of every item and modifier, empty states, decodability of every modifier `arg`. |
| `tests/plist_test.rs`    | `workflow/info.plist`: every keyword (`bw*`) and the subcommand it runs, every modifier connected to the action, every keyword in the workflow readme (uses macOS `plutil`). |

## Project layout

```
.
├── src/                    Rust sources (see docs/architecture.md)
├── tests/                  Integration tests + shared FakeRunner
├── workflow/
│   ├── info.plist          Alfred workflow definition (the binary is added at build time)
│   └── icon.png            Workflow icon, converted from Brave's app icon
├── scripts/
│   ├── build.sh            Build + package the .alfredworkflow
│   ├── deploy.sh           Build + install into Alfred
│   └── icon.sh             Regenerate workflow/icon.png
├── docs/                   This documentation
├── .github/workflows/      CI and release-please pipelines
├── release-please-config.json
└── .release-please-manifest.json
```

## Editing the Alfred graph

`workflow/info.plist` defines six Script Filters (one per keyword), each
connected — for every modifier used — to one Run Script, connected to a Post
Notification. If you edit the workflow in Alfred's UI, copy the updated
`info.plist` from the installed workflow folder back into `workflow/` before
committing (the next `./scripts/deploy.sh` overwrites the installed one).
Validate it with:

```sh
plutil -lint workflow/info.plist
```
