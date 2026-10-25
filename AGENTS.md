# Working on teamzones

## The dev loop

**1. Open an issue.** Every change starts as a GitHub issue on `paulchiu/teamzones-tui`, created with `gh issue create`. Write the body for the agent that will pick it up: what changes, the acceptance criteria as observable behaviour, and nothing already obvious from the code.

**2. Branch and build test first.** Branch from an up-to-date `main` as `feat/<N>-<slug>` (or `fix/<N>-<slug>`). Drive the change one acceptance criterion at a time: a test that fails for the right reason, then the least code that passes it.

**3. Ship it.** Run `just ci` before you push, not after. Open the PR with a body that ends `Closes #N`. Every PR carries exactly one of the labels `major`, `minor`, `patch` or `norelease`, and the `Enforce Semantic Versioning` check fails the PR until it does. Merging a labelled PR bumps the version by that label, rolls the changelog and tags `vX.Y.Z`; `norelease` merges and moves nothing. Never run `cargo set-version` and never commit a version bump: the version is the tag pipeline's to move.

The label is semantic. `minor` is for anything a user could not do before, and for a breaking change while the major is held at 0. `patch` is for a fix or a tightening. `norelease` is only for a docs or CI change no user can observe. When in doubt, `patch`. Never apply `major`.

## The checks a PR gets

A PR gets 13 checks: `Format, Lint, Test & Build (ubuntu-26.04)`, `Format, Lint, Test & Build (macos-latest)`, `MSRV`, `Workflow parse`, `CI`, `Enforce Semantic Versioning`, `plan`, `build-local-artifacts`, `build-global-artifacts`, `host`, `publish-homebrew-formula`, `announce` and `dependabot-auto-merge`. `plan` is dist checking that the release workflow still matches `dist-workspace.toml`; the five release jobs after it run only on a tag, and `dependabot-auto-merge` only on a PR Dependabot opened, so all six are skipped on any other PR. Green means all 13 registered, the seven others passed and only those six were skipped. A wait loop that only checks for nothing pending reports green before the checks exist, so count them. Compare `gh pr view --json headRefOid` with `git rev-parse HEAD` before trusting a run.

## What you need to know first

The workspace is two crates. `crates/teamzones-core` computes everything a screen or CLI line needs and knows nothing about rendering; `crates/teamzones` is the TUI and the headless CLI. `just check-core-isolation` holds that boundary with an allowlist of the core's direct dependencies, each paired with its reason. No terminal, rendering or clipboard crate (`ratatui`, `crossterm`, `arboard`) ever joins it.

`just ci` is the gate: `fmt-check lint test docs check-core-isolation build`. The GitHub workflow runs the same recipe, so a green run here is a green pipeline.

[docs/engineering.md](docs/engineering.md) holds the engineering conventions and the lessons from repon and blubat, with their sources. Read it before your first change. Where it and a generic Rust guide disagree, it wins.

[GLOSSARY.md](GLOSSARY.md) defines the project's words, and they are used in those senses in the code as well as the docs.

Agents never run the TUI interactively. Run the binary only with stdin from `/dev/null` (`teamzones say team 3pm --no-copy </dev/null`), and exercise the TUI through `TestBackend` and the pty test.

## Where things are written down

**[docs/spec/](docs/spec/)** is the product contract: [product](docs/spec/product.md), [config](docs/spec/config.md), [time](docs/spec/time.md), [copy](docs/spec/copy.md), [cli](docs/spec/cli.md), [screen](docs/spec/screen.md), [keys](docs/spec/keys.md) and [theme](docs/spec/theme.md). Tests parse it rather than retype it: the golden rows come from `copy.md` and `cli.md`, the keymap table in `keys.md` is compared with `BINDINGS`, and the fenced example in `config.md` must equal the fixture byte for byte. Editing spec prose can therefore turn CI red, and that is the point. Never change the contract inside a feature PR: contract changes go back to Paul.

**[docs/decisions.md](docs/decisions.md)** says what was chosen and why. A PR that makes or reverses a decision appends an entry (`## D<n>: <invariant> (YYYY-MM-DD)` with `Decision`, `Why` and `Held by` lines), and a PR that adds the test holding a decision fills that entry's `Held by` line. Never rewrite an entry in place: a reversal is a new entry, and the old one gains `Superseded by D<n>`.

**[CHANGELOG.md](CHANGELOG.md)** says what changed, for someone installing the tool. Every PR with a user-visible change adds its entries under `## [Unreleased]` in the same PR, under `### Added`, `### Changed`, `### Fixed` or `### Removed`, adding only the subsections it needs. One change per bullet, written for a user, with no per-bullet dates. Never rename the heading yourself: the version-tag workflow renames `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD` (UTC) on merge and inserts a bare `## [Unreleased]` above it, and it fails if the heading is missing.

## Constraints

- No em dashes anywhere (code, docs, commits, PRs), and Australian spelling in prose. Fictional names only, apart from Paul himself in the fixture.
- Stage each path by name. Never `git add -A` or `git add .`.
- No `unwrap` or `panic!` in production code, no `unsafe`, and `#[expect(lint, reason = "...")]` rather than `#[allow]`.
- Add no dependency without a line in the PR description saying why. Every shared dependency is already declared in the root `[workspace.dependencies]`.
- When uncertain, stop and explain the ambiguity in the PR or an issue rather than guessing. Report a false premise instead of implementing around it.
