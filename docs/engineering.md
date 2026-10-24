# Engineering

The engineering conventions and lessons from the teamzones build spec (2026-10-24), verbatim. Every change to this repo follows them. Sandbox paths are Paul's local notes and are not published.

## Engineering conventions

Every agent gets these and the [lessons](#lessons-from-repon-and-blubat) verbatim. Paul maintains this code himself and is new to Rust, so prefer clear idiom over clever code, and explain any non-obvious idiom in the PR description.

* **Style**: small surface area, a functional style, pure cores with thin I/O shells, and `Result`/`Option` combinators over nested branching. Keep call chains shallow and the code self-documenting. Comments are one sentence on the contract plus at most one short "why". Follow the `tdd` skill's `references/functional-style.md` and `references/test-quality.md`.
* **Architecture**: Elm-style. `App` state, a `Msg` enum, a pure `update(state, msg) -> (state, Vec<Effect>)` and a pure `view(&state, frame)`. Effects (clipboard, file writes, editor, timers) run in one layer, like blubat's `effects.rs` ([GitHub](https://github.com/paulchiu/blubat/blob/8b4b0d5f32dcff25193abb8c8ba0a5234b914253/blubat/src/effects.rs#L37)). `teamzones-core` computes everything a screen or CLI line needs and knows nothing about rendering (repon ADR 0005, [GitHub](https://github.com/paulchiu/repon/blob/a4b26cc543af925c8529157ca24b0855c01210d5/docs/adr/0005-rendering-agnostic-core.md)).
* **One binding table** feeds the footer, the help overlay and the dispatcher. Start from blubat's shape ([GitHub](https://github.com/paulchiu/blubat/blob/8b4b0d5f32dcff25193abb8c8ba0a5234b914253/blubat/src/tui/app.rs#L21-L72)), and add a context and a footer priority:

  ```
  pub struct Binding {
      pub keys: &'static str,
      pub label: &'static str,
      pub hinted: bool,
      pub context: Context,      // 👈 new: Main, Matrix, Prompt, CopyMenu, SortMenu
      pub priority: u8,          // 👈 new: footer drop order, lower drops first
  }
  ```
* **Crates**: edition 2024, `rust-version = "1.95"`.
  + Shared dependencies, all declared in M0's `[workspace.dependencies]`: `chrono` 0.4, `chrono-tz` 0.10, `serde`, `toml`, `toml_edit` 0.25, `thiserror`.
  + The binary adds `ratatui` 0.30, `crossterm` 0.29, `clap` 4 (derive), `arboard` 3, `etcetera`, `iana-time-zone` and `color-eyre`.
  + Dev dependencies: `assert_cmd` and `insta` (frame snapshots).
  + Copy blubat's `[workspace.lints]` block ([GitHub](https://github.com/paulchiu/blubat/blob/8b4b0d5f32dcff25193abb8c8ba0a5234b914253/Cargo.toml#L5-L35)), including `unsafe_code = "deny"`, and lint with `--all-targets --all-features --locked`. See [Rust practice](#lessons-from-repon-and-blubat) for what the table leaves out on purpose.
  + Add no other dependency without a line in the PR description saying why.
* **Tests**: unit tests next to the code in core. CLI tests use `assert_cmd` against the fixture with `--now`. TUI tests use `TestBackend`, with structural assertions (the row of each block, the say line text, labels) plus `insta` snapshots of the full frame. Snapshots never contain the version string, because they must survive release bumps. Spec tables are parsed by tests rather than retyped into them (see [Tests are the enforcement](#lessons-from-repon-and-blubat)).
* **Gate**: `just ci` = `fmt-check lint test docs check-core-isolation build`, modelled on repon's justfile ([GitHub](https://github.com/paulchiu/repon/blob/a4b26cc543af925c8529157ca24b0855c01210d5/justfile#L199)). `check-core-isolation` runs `cargo tree -p teamzones-core --edges normal --depth 1` against an allowlist that pairs each crate with a reason.
* **Process**: a GitHub issue first (`gh issue create`), then a branch named `feat/<N>-<slug>`. Commit and open the PR with the `git-commit` and `gh-pr` skills (`~/.claude/skills/`). The PR body says `Closes #N`. Apply exactly one version label. Never run `cargo set-version` and never commit a version bump, because the tag pipeline owns the version. `gh-pr` creates draft PRs, so run `gh pr ready` before merging. Every PR with a user-visible change adds its entry under `## [Unreleased]` in `CHANGELOG.md`. A PR that makes or reverses a decision appends it to `docs/decisions.md`, and a PR that adds the test holding a decision fills that entry's `Held by` line (see [Changelog and decision log](#lessons-from-repon-and-blubat)).
* **Writing**: no em dashes anywhere (code, docs, commits, PRs), and Australian spelling in prose. Use fictional names only. The fixture's Paul is the user himself, and everyone else in it is fictional.
* **When uncertain**: stop and explain the ambiguity in the PR or an issue, or propose a short plan, rather than guessing. Report a false premise instead of implementing around it. Never edit the shared contract inside a milestone PR. Contract changes go back to Paul.

## Lessons from repon and blubat

What Paul's last two Rust TUIs learned the hard way, taken from the sandbox archive and the two repos' recent history. Every agent reads this with the engineering conventions. Where a rule here and a generic Rust guide disagree, this section wins.

### Sources

The notes live under `/Users/paul/dev/sandbox/archive/`. Read the one a rule cites before arguing with the rule.

| Note | What to take from it |
| --- | --- |
| blubat Rust best practices review (`~/dev/sandbox/archive/2026-08/2026-08-21/2026-08-21 blubat Rust best practices review.md`) | Apollo's handbook applied to blubat: the curated lint table, `#[expect]`, `# Errors` docs, and the findings it rejected and why. |
| blubat unsafe Rust usage report (`~/dev/sandbox/archive/2026-08/2026-08-14/2026-08-14 blubat unsafe Rust usage report.md`) | `unsafe_code = "deny"` as a compiler guarantee, proved with a negative test. |
| blubat deep-module architecture workflow (`~/dev/sandbox/archive/2026-08/2026-08-21/2026-08-21 blubat deep-module architecture workflow and draft PR.md`) | Classify once behind one function. Pin each workflow stage's model. |
| blubat implementation M0 to M3 (`~/dev/sandbox/archive/2026-08/2026-08-02/2026-08-02 blubat implementation M0 to M3.md`) | The milestone workflow this build copies (implement, three reviewers, fix, merge), and the editor admission gate. |
| blubat TUI feedback round (`~/dev/sandbox/archive/2026-08/2026-08-04/2026-08-04 blubat TUI feedback round ships v0.8.2 to v0.10.0.md`) | Modal menus that answer their own keys, footer fitting, and snapshots that survive a version bump. |
| blubat editor typeahead leak (`~/dev/sandbox/archive/2026-08/2026-08-15/2026-08-15 blubat editor typeahead leak.md`) | Drain the terminal after an editor returns. |
| blubat dependabot dist pin (`~/dev/sandbox/archive/2026-08/2026-08-15/2026-08-15 blubat dependabot checkout v7 dist pin.md`) | Dependabot against a dist-generated `release.yml`. |
| blubat descriptor leak measurements (`~/dev/sandbox/archive/2026-09/2026-09-21/2026-09-21 Blubat descriptor leak measurements.md`) | A test whose tolerance hides the defect it exists to catch. |
| Repon handover notes (`~/dev/sandbox/archive/2026-09/2026-09-02/2026-09-02 Repon Handover Notes.md`) | Standing rules, CI traps and the three recurring defect species. |
| Repon ADR audit (`~/dev/sandbox/archive/2026-09/2026-09-05/2026-09-05 Repon ADR Audit.md`) | A restriction survives only when a test holds it. |
| Repon Rust refactoring plan (`~/dev/sandbox/archive/2026-09/2026-09-06/2026-09-06 Repon Rust refactoring plan for Claude Code.md`) | Four Rust guides and their limits, one value instead of paired options, and red for the right reason. |
| Choreday `AGENTS.md` and `release.yml` ([GitHub](https://github.com/paulchiu/obsidian-choreday/blob/609374292f5113a6b48589699c556dfcfe7cdf1d/AGENTS.md#L127)) | The changelog workflow this build copies: entries under `[Unreleased]` in each PR, rolled into the version heading by the release job. |

### Rust practice

* **Lints are curated, not wholesale.** Copy blubat's table as it stands ([GitHub](https://github.com/paulchiu/blubat/blob/8b4b0d5f32dcff25193abb8c8ba0a5234b914253/Cargo.toml#L5-L35)). Do not turn on `clippy::pedantic` or `clippy::nursery`: on blubat they measured 803 warnings, mostly `use_self`, `doc_markdown` and `must_use_candidate`, which bury the real findings (Sandbox: `~/dev/sandbox/archive/2026-08/2026-08-21/2026-08-21 blubat Rust best practices review.md`). Lint the way blubat's justfile does, `cargo clippy --all-targets --all-features --locked -- -D warnings` ([GitHub](https://github.com/paulchiu/blubat/blob/8b4b0d5f32dcff25193abb8c8ba0a5234b914253/justfile#L20-L21)).
* **No `unwrap` or `panic!` in production code.** `expect` is allowed only on a genuine invariant, and its message names the invariant. Tests may unwrap.
* **`#[expect(lint, reason = "...")]`, never `#[allow]`**, so a suppression warns once nothing needs it.
* **Public fallible functions get an `# Errors` section** (`missing_errors_doc` is in the table). Do not add `#![deny(missing_docs)]`: blubat rejected it because it would mean 90 comments restating field names, against the comment rule.
* **No unsafe.** `unsafe_code = "deny"` stays, with no exemptions. teamzones has no FFI of its own (arboard and crossterm own theirs), unlike blubat, which confines unsafe to seven FFI files (Sandbox: `~/dev/sandbox/archive/2026-08/2026-08-14/2026-08-14 blubat unsafe Rust usage report.md`). If something seems to need unsafe, stop and ask.
* **The cast lints stay off**, as on blubat, where every hit was provably bounded. Keep `as` casts inside the layout and slot helpers, where tests cover their bounds.
* **Generic guides are reference, not spec.** The repon refactoring plan read Apollo's `rust-best-practices` ([skills.sh](https://www.skills.sh/apollographql/skills/rust-best-practices)), `rust-async-patterns`, `rust-testing` and `rust-patterns` (Sandbox: `~/dev/sandbox/archive/2026-09/2026-09-06/2026-09-06 Repon Rust refactoring plan for Claude Code.md`). Take the principles. Do not add mockall, rstest, proptest, Criterion, tokio or a coverage target because a guide shows them. Apollo's pointer table oversimplifies `Send` and `Sync`, so use the standard library's definitions. Never raise `rust-version` to reach a newer feature.
* **Two deliberate differences from blubat.** Core uses `thiserror`, where blubat hand-writes `Display` to hold its core to six crates, and the binary snapshots frames with `insta`, where blubat hand-rolls them. Both sit in the allowlist or dev dependencies with a reason. Neither is a precedent for a third crate.

### Modelling

* **Match exhaustively, never hand-enumerate.** This is the first of repon's three recurring defect species (Sandbox: `~/dev/sandbox/archive/2026-09/2026-09-02/2026-09-02 Repon Handover Notes.md`). `match` on `Band`, `Level`, `Question`, `Context` and `Msg` with no wildcard arm. Destructure a struct in any function that must see every field (validation, write-back, copy), so the compiler finds the next variant or field.
* **Classify once.** blubat re-derived a device's status in four files until PR #45 put it behind `App::status` ([GitHub](https://github.com/paulchiu/blubat/blob/8b4b0d5f32dcff25193abb8c8ba0a5234b914253/blubat/src/tui/status.rs#L105)) (Sandbox: `~/dev/sandbox/archive/2026-08/2026-08-21/2026-08-21 blubat deep-module architecture workflow and draft PR.md`). Here core decides each person's band, each place's level and whether a slot fits. The view maps each value to a glyph and a style through one exhaustive match, and never decides it again.
* **One value, not paired options.** repon's notice was two `Option` fields that had to agree (R6 in the refactoring plan). The notice row here is `Option<Notice { text, raised_at }>`. Whether it shows is decided from a sampled now and the current timeout, so a reload that changes the timeout applies to a notice already showing. The same goes for a range (`Range { start, end }` over `Slot`) and an undo entry (one value holding the file before, the state before and the expiry).
* **Absent is not unreadable.** blubat shipped two fixes (#75 and #76) where an unreadable lock or heartbeat answered as "nothing there" ([GitHub](https://github.com/paulchiu/blubat/blob/8b4b0d5f32dcff25193abb8c8ba0a5234b914253/blubat/src/lock.rs#L107)) ([GitHub](https://github.com/paulchiu/blubat/blob/8b4b0d5f32dcff25193abb8c8ba0a5234b914253/blubat-core/src/health.rs#L82)). A missing config means defaults plus the first-run notice. A config or state file that exists but cannot be read or parsed is an error naming the path, and never falls back to defaults. Nothing writes on top of a read that failed.
* **Order comes from the file, not a hash.** Rows, groups and copy order follow config order, held in a `Vec`. Never iterate a `HashMap` into output, and never write a test that assumes an order the code does not define (repon #484).
* **Width is display width.** Truncate city and label text by display columns, using ratatui's `Span::width`, through one helper with an ascii-safe mark like repon's `truncate_with_mark` ([GitHub](https://github.com/paulchiu/repon/blob/a4b26cc543af925c8529157ca24b0855c01210d5/crates/repon/src/components/list.rs#L665)). Never truncate by bytes or chars, because `✈`, `⌂` and `☼` are on screen.
* **Re-clamp the cursor whenever its list shrinks.** repon fixed this twice in one PR (#493). Group switches, folding on resize, reload, delete and the matrix all re-clamp focus in `update`, with a test each.

### Terminal

* **Release what you enable** (repon ADR 0024). Raw mode, the alternate screen and mouse capture are restored on quit, on editor handoff and in the panic hook. Prove it with a pty test modelled on repon's `terminal_restoration.rs`, whose `classify_pty_read` accepts both macOS's `Ok(0)` and Linux's `EIO` as the end of output ([GitHub](https://github.com/paulchiu/repon/blob/a4b26cc543af925c8529157ca24b0855c01210d5/crates/repon/tests/terminal_restoration.rs#L264)).
* **Park the key reader while the editor owns the terminal, and drain on return.** blubat's reader raced vim for stdin until an admission gate parked it ([GitHub](https://github.com/paulchiu/blubat/blob/8b4b0d5f32dcff25193abb8c8ba0a5234b914253/blubat/src/tui/events.rs#L24-L33)). After vim exited, late OSC colour replies were then read as keys, and their `/` opened the filter, until `drain_typeahead` discarded buffered input plus a 20 ms grace window ([GitHub](https://github.com/paulchiu/blubat/blob/8b4b0d5f32dcff25193abb8c8ba0a5234b914253/blubat/src/tui/terminal.rs#L88)) (Sandbox: `~/dev/sandbox/archive/2026-08/2026-08-15/2026-08-15 blubat editor typeahead leak.md`). repon #490 made the same handoff testable with an injected opener and a pure function that builds the editor argv.
* **Agents never run the TUI interactively.** Run the binary only with stdin from `/dev/null` (`teamzones say team 3pm --no-copy </dev/null`). Exercise the TUI through `TestBackend` and the pty test. The "Manually" lines in Verification are for the orchestrator's final review or Paul, not for implementers.

### Tests are the enforcement

* The repon ADR audit checked 423 claims across 33 records and found that a restriction survived when, and only when, a test held it (Sandbox: `~/dev/sandbox/archive/2026-09/2026-09-05/2026-09-05 Repon ADR Audit.md`). Glyphs, chords and column widths with no test drifted. Exhaustive matches, compile-time glyph checks and tests that parse the spec held.
* **The spec is parsed, not retyped.** The second defect species is production and tests sharing one source of truth with nothing pinning it to the design of record. So a test parses the keymap table in `docs/spec/keys.md` and compares it with `BINDINGS` row for row, like repon's `compiled_table_matches_the_spec_default_map_row_for_row` ([GitHub](https://github.com/paulchiu/repon/blob/a4b26cc543af925c8529157ca24b0855c01210d5/crates/repon/src/keys.rs#L2352)). Another asserts that the fenced example in `docs/spec/config.md` equals the fixture byte for byte. The golden tests read their rows from `docs/spec/copy.md` and `docs/spec/cli.md`. Editing spec prose can therefore turn CI red, and that is the point.
* **Meta-tests on the binding table**, from blubat ([GitHub](https://github.com/paulchiu/blubat/blob/8b4b0d5f32dcff25193abb8c8ba0a5234b914253/blubat/src/tui/app.rs#L1126-L1237)): every hinted key dispatches, no key the footer advertises is typed into a prompt, and each modal context acts on its own keys and swallows the rest.
* **Prove each guard once by breaking it.** The third species is a check that quietly stops checking. Add a forbidden crate and watch `check-core-isolation` fail. Add an unsafe block and watch the build fail, as blubat PR #33 did. Drop a binding row and watch the spec test fail. Record each negative run in the PR description, and do not commit it. Size any tolerance to the defect: blubat's descriptor test (20 runs, tolerance 8) could not see a leak on 5% of calls (Sandbox: `~/dev/sandbox/archive/2026-09/2026-09-21/2026-09-21 Blubat descriptor leak measurements.md`).
* **Snapshots are version-safe.** blubat's pinned overlay frame hard-coded `v0.8.1` and would have failed on every release bump (Sandbox: `~/dev/sandbox/archive/2026-08/2026-08-04/2026-08-04 blubat TUI feedback round ships v0.8.2 to v0.10.0.md`). Build expected version text from the same `env!("CARGO_PKG_VERSION")` the render reads, or keep it out of snapshots.
* **Red for the right reason.** A new test fails on an assertion about behaviour, not on a missing symbol, a fixture that never started or a timeout. A characterisation test that passes first time takes the `tdd` skill's explicit waiver, with a reason.
* **Seams are per instance.** `Clock`, `Clipboard`, the terminal and the editor opener are injected into each `App`. Never use a global mutable flag shared across parallel tests, and never mock teamzones' own modules.

### Changelog and decision log

* Two plain files and no ADR directory. `CHANGELOG.md` says what changed, for someone installing the tool. `docs/decisions.md` says what was chosen and why, for someone changing the code. repon's ADR system (a genre split, numbered records and an enforcement test) is deliberately not copied.
* **Changelog**, as Choreday does it ([GitHub](https://github.com/paulchiu/obsidian-choreday/blob/609374292f5113a6b48589699c556dfcfe7cdf1d/AGENTS.md#L127)). Keep a Changelog 1.1.0 format ([Keep a Changelog](https://keepachangelog.com/en/1.1.0/)). Each PR with a user-visible change adds its entries under `## [Unreleased]` in the same PR, in `### Added`, `### Changed`, `### Fixed` or `### Removed`. Entries are written for a user, one change per bullet, with no per-bullet dates. Agents never rename the heading: the bump commit in `version-tag.yml` renames `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD` (UTC) and inserts a fresh `## [Unreleased]` above it, like Choreday's roll step ([GitHub](https://github.com/paulchiu/obsidian-choreday/blob/609374292f5113a6b48589699c556dfcfe7cdf1d/.github/workflows/release.yml#L118-L146)). A `norelease` PR may add entries that ship with the next bump.
* One difference from Choreday: the fresh `[Unreleased]` block is a bare heading, and a PR adds only the subsections it needs. dist builds the GitHub release notes from the version's section as written, so empty subsections would show up in the release, whereas Choreday strips them in its own extract step.
* **Decision log.** One append-only file. Each entry is a heading such as `## D13: The core never renders (2026-10-24)` followed by three short lines: `Decision`, `Why` and `Held by`. `Held by` names the test, lint or `just` recipe that holds the decision, or says `nothing yet`. It is the one part of repon's system worth keeping: the audit found a restriction survived when, and only when, a test held it, and the line keeps that question in view without a meta-test.
* Record invariants, not values. "Switching groups never moves a block" survives. "The where block is six rows" does not. Detail that moves (glyphs, widths, copy) lives in `docs/spec/`, where tests parse it, and the entry links that section instead of restating it.
* Hands-on use beats reasoning on product calls. Every one of the five repon product decisions that were reversed fell on first use, within two days. When Paul's feedback contradicts an entry, append a new entry that supersedes it, and add `Superseded by D<n>` under the old heading, keeping its reasoning. Never rewrite a decision in place.

### Process and CI traps

* **Green means every expected check registered and passed.** A wait loop that only checks for no pending checks reports green before the checks exist. M0 counts the checks a PR gets and writes the number into `AGENTS.md`.
* **`gh pr view` can serve a stale head.** Compare `gh pr view --json headRefOid` with `git rev-parse HEAD` before trusting a green run.
* **A CI-only failure is not assumed to be a flake.** Two of repon's were real defects (pty end-of-file semantics and a generation race). Linux and macOS disagree about pty end of file.
* **zsh has no `PIPESTATUS`.** Read exit codes from `$?`, with output redirected to a log.
* **Stage each path by name.** Never `git add -A` or `git add .`.
* **Labels are semantic.** Use `minor` for anything a user could not do before, and for a breaking change while the major is held at 0. Use `patch` for a fix or a tightening, and `norelease` only for docs or CI changes no user can observe. When in doubt, use `patch`. Never apply `major`.
* **Report a false premise.** If a card or this spec asks for something the code or the maths contradicts, say so in the PR or an issue rather than implementing around it. Three repon issues carried an error that implementers caught this way.
* **Read only outside the worktree.** Never fetch, check out, commit or clean another repository under `~/dev` or `~/dev-misc`. The reference repos are read only. Never bulk-clean shared machine state.
* **Pin each workflow stage's model.** blubat's architecture workflow left four stages on the session default and spent about 770k tokens.
