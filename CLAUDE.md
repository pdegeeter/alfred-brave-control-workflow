# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Keep everything in sync (always)

Every code change must update its tests, the docs under `docs/`, the `README.md`,
and this `CLAUDE.md` in the **same** change — never as a follow-up. If you change
behaviour (a keyword, a modifier, an action verb, a JXA script, the history
query, the plist, the release flow), find and fix every reference to the old
behaviour across `README.md`, `docs/`, `CLAUDE.md`, `workflow/info.plist`'s
`readme`, and doc comments before committing. `cargo test --all`,
`cargo clippy --all-targets -- -D warnings`, and `cargo fmt --all` must all pass.

## What this is

An Alfred (macOS) workflow written in Rust that controls Brave Browser, modelled
on the Safari Control workflow. A single binary, `alfred-brave`, backs every
object of the Alfred graph:

- Six **Script Filters**, one per keyword: `window` (`bw`), `profiles` (`bwp`),
  `tabs` (`bwt`), `history` (`bhi`), `current` (`bwc`), `private-tab` (`btp`).
  They print Alfred JSON.
- One **Run Script**, `alfred-brave run <arg>`, connected to every script filter
  for every modifier used. `arg` is an encoded `action::Action`
  (`"<verb> <payload>"`, see `docs/architecture.md`). Whatever it prints feeds a
  Post Notification.

The binary ships inside `workflow/` (with `info.plist`) and is packaged into a
`.alfredworkflow` file for distribution.

## Commands

```sh
cargo test --all                          # full suite (offline, no Brave needed)
cargo test --test action_test             # one test file
cargo clippy --all-targets -- -D warnings # lint; warnings are errors (CI gate)
cargo fmt --all                           # format (CI checks with --check)

./scripts/build.sh            # sync plist version + build release + package .alfredworkflow
./scripts/deploy.sh           # build + symlink workflow into Alfred (live dev)
./scripts/deploy.sh --import  # build + open the .alfredworkflow to import it

# Exercise the binary directly (it speaks Alfred JSON on stdout):
cargo build && ./target/debug/alfred-brave tabs rust | python3 -m json.tool
```

`powerpack-cli` is required for build/deploy/packaging: `cargo install powerpack-cli`.

## Architecture

**Everything is a pure function except one trait.** Every process spawn —
`osascript` (JXA, clipboard, Alfred), `/usr/bin/sqlite3` (history) and
`/usr/bin/open` (profiles) — goes through `runner::CommandRunner`
(`SystemRunner` in prod). Tests inject `FakeRunner` (`tests/common/mod.rs`),
which returns queued canned stdout per program and records invocations, so the
suite runs offline. Keep new logic pure and route any new spawn through it.

- `brave.rs` — fixed JXA scripts (inputs via `argv`, never interpolated) for
  listing tabs, the current tab, focus, close, new window/tab. Listing scripts
  return `null` instead of launching Brave when it is not running. Profiles are
  opened with `open -na "Brave Browser" --args --profile-directory=<dir>
  --new-window [url]` (not scriptable through AppleScript).
- `profiles.rs` — parses `profile.info_cache` from Brave's `Local State`,
  ordered by `active_time`. Missing/malformed file ⇒ no profile, never an error.
- `history.rs` — per profile, `sqlite3 -readonly -json
  "file:<dir>/History?immutable=1" "<SQL>"`; terms become `LIKE … ESCAPE '\'`
  conditions (quotes doubled); results are merged keeping each URL's latest
  visit. A failing profile is skipped unless all fail.
- `query.rs` — `bw` query: blank, `xN` (capped at 20), URL / bare host, or a
  Brave Search.
- `action.rs` — the `arg` protocol (`Action::to_arg` / `Action::parse`) and
  `execute`.
- `alfred.rs` — item builders for every keyword.

## Conventions that matter here

- **The `arg` protocol**: only the last payload field may contain spaces.
  Adding a verb means updating `to_arg`, `parse`, `execute`, the round-trip test
  in `tests/action_test.rs` and the table in `docs/architecture.md`.
- **Modifier connections**: every script filter has one connection per
  modifier used (⌘, ⌥, ⌃, ⇧⌘) to the Run Script. Adding a new modifier
  combination means adding its connection in `workflow/info.plist`.
- **No `uid` on Alfred items** — Alfred would re-sort them. Filtering and
  ordering are done in Rust (`alfredfiltersresults = false`).
- **Script filters never hard-fail**: errors become an invalid Alfred item.
- **Keywords are fixed** (`bw`, `bwp`, `bwt`, `bhi`, `bwc`, `btp`); the `edit`
  action reopens Alfred on `bw <url>` (`action::WINDOW_KEYWORD`).
- **Editing `workflow/info.plist`**: if you edit the workflow in Alfred's UI
  while symlinked, copy the updated `info.plist` back into `workflow/` before
  committing, and validate with `plutil -lint workflow/info.plist`. The binary
  in `workflow/` is a build artifact (gitignored); `info.plist` and `icon.png`
  are tracked.
- **Workflow icon**: `workflow/icon.png` is converted from Brave's app icon by
  `scripts/icon.sh` (`sips`). It is committed and not regenerated at build time.
- **Protected `main`**: work via PRs only (enforced for admins). CI `test` and
  `package` must be green, and **all commits must be signed** (SSH signing) to
  merge. See `docs/development.md`.

## Releases

Automated via release-please (`release-type: rust`). Land Conventional Commits
on `main` (`feat:`/`fix:`/`chore:`…); release-please opens a Release PR that
bumps `Cargo.toml`/`Cargo.lock` and `CHANGELOG.md`. Merging it tags `vX.Y.Z`,
creates the GitHub Release, and the `build-and-upload` job attaches the
`.alfredworkflow` (arm64) as a release asset. Do not bump versions by hand.
See `docs/release.md`.

Target platform is **macOS arm64 only**; CI and release build on `macos-latest`.
