# Decisions

What was chosen and why, for someone changing the code. Append only: a reversed decision gets a new entry that supersedes it, and the old heading gains a `Superseded by D<n>` line, keeping its reasoning. Record invariants, not values. Detail that moves (glyphs, widths, copy) lives in [docs/spec/](spec/), where tests parse it, and an entry links that section instead of restating it.

Each entry has three lines. `Held by` names the test, lint or `just` recipe that holds the decision, or says `nothing yet`.

## D1: A region is a load, not a clock (2026-10-24)

Decision: `teamzones say apps 3pm` prints `3pm AEDT, off-peak in AU, UK and US`. Places add a trading state to the say line, never a zone, and `--zones` (and `c z`) gives the old `ttz` shape.

Why: Paul picked concept V3-B, which makes this the deciding call.

Held by: nothing yet

## D2: The where block never collapses (2026-10-24)

Decision: The where block's height is fixed by the whole roster, and small terminals fold rows instead.

Why: Switching groups never moves anything on screen.

Held by: nothing yet

## D3: Home counts in every group that asks who (2026-10-24)

Decision: Home is counted whenever the marks ask who. In a places-only group, home is the axis only.

Why: A deploy should not be proposed while Paul is off.

Held by: nothing yet

## D4: Load comes from hand-written trading profiles only (2026-10-24)

Decision: A place's load is read from its `trading` profile and nothing else.

Why: No data source exists yet.

Held by: nothing yet

## D5: One zone per region (2026-10-24)

Decision: Each place has exactly one zone. Split a region by adding places (`US-E`, `US-W`). See [config](spec/config.md#data-model-and-config).

Why: Cheap in this model, and nothing needs a union yet.

Held by: nothing yet

## D6: Off-peak means not at peak (2026-10-24)

Decision: `where = "off-peak"` (the default) counts a place that is not at peak. `where = "quiet"` counts a place only while it is closed.

Why: With the sample profiles, AU, UK and US are never all closed at once.

Held by: nothing yet

## D7: Names and paths (2026-10-24)

Decision: Repo `paulchiu/teamzones-tui`, crates `teamzones` and `teamzones-core`, binary `teamzones`, Homebrew formula `teamzones`, MIT licence, public. Config and state paths are in [config](spec/config.md#files-state-and-lifecycle).

Why: "teamzones TUI" as agreed. The old project's binary is `tz`, so nothing collides.

Held by: `crates/teamzones/tests/version.rs` holds the binary name; nothing yet holds the paths

## D8: Mouse capture is on by default (2026-10-24)

Decision: The TUI captures the mouse at start, and `M` releases it for text selection.

Why: The slider is a mouse surface. This is the deliberate opposite of repon ADR 0024 ([GitHub](https://github.com/paulchiu/repon/blob/a4b26cc543af925c8529157ca24b0855c01210d5/docs/adr/0024-repon-releases-what-it-enables-and-holds-mouse-capture-off.md)).

Held by: nothing yet

## D9: Clipboard falls back from arboard to OSC 52 to the notice row (2026-10-24)

Decision: Copying tries `arboard` first, then OSC 52. If both fail, the notice row shows the text. See the notice row in [screen](spec/screen.md).

Why: mossmd already uses arboard, and OSC 52 reaches the local machine over SSH.

Held by: nothing yet

## D10: say prints and copies (2026-10-24)

Decision: `teamzones say` prints the line to stdout and copies it unless `--no-copy` is given or stdout is not a TTY. When it copies, `📋 copied` goes to stderr.

Why: Keeps the `ttz` habit and keeps stdout clean for scripts.

Held by: nothing yet

## D11: Travel is set in config, never inferred (2026-10-24)

Decision: Travel is `[travel] here` plus `until`, written by `teamzones here`. It is never inferred from the system time zone.

Why: Paul's Mac reports `Australia/Brisbane` all year.

Held by: nothing yet

## D12: Time is computed on a 15-minute grid (2026-10-24)

Decision: Range edges, `n` targets and range states are all evaluated on 15-minute slots aligned to UTC quarter hours.

Why: The screen and the jump agree exactly at 120 columns.

Held by: nothing yet

## D13: The core never renders (2026-10-24)

Decision: `teamzones-core` computes everything a screen or CLI line needs and depends on no terminal, rendering or clipboard crate.

Why: A rendering-agnostic core can be tested with plain values, as in repon ADR 0005 ([GitHub](https://github.com/paulchiu/repon/blob/a4b26cc543af925c8529157ca24b0855c01210d5/docs/adr/0005-rendering-agnostic-core.md)).

Held by: `just check-core-isolation`

## D14: One binding table feeds the footer, help and dispatch (2026-10-24)

Decision: Every key's footer hint, help line and dispatch come from one `BINDINGS` table.

Why: Hints that are separate strings drift from what the keys do.

Held by: nothing yet (the TUI milestone adds the keymap spec test and the binding meta-tests)

## D15: update is pure and effects run in one layer (2026-10-24)

Decision: `update(state, msg)` returns the new state and a list of effects. Clipboard, file writes, the editor and timers run only in the effects layer.

Why: A pure update is testable without a terminal, a clipboard or a file system.

Held by: nothing yet (the TUI milestone adds a source-scan test that the update module names no I/O crate)
