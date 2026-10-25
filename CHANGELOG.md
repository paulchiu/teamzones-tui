# Changelog

This file documents notable changes to the project.

The format is based on [Keep a Changelog v1.1.0](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.8.0] - 2026-10-25

### Added

- A world map above the timeline shows the night side and the sun for the time you are viewing, with a marker for everyone and every place, labelled with each counted city's local time and each counted place's state, as in `UK closed 3-5am`.
- The map shows when the terminal has at least 100 columns and 12 spare rows. `m` hides it, or opens it in a box over the timeline when there is no room, and `esc` closes the box.
- The mouse works: the wheel moves the time (shift for an hour, sideways for a day), dragging draws a range or moves it and its ends, clicking a fitting run jumps to it, and clicking a row, a mark, a group name, the legend, a block title, `y copy`, `u undo`, `◂` or the header's now does what its key does. Clicking outside a dialog closes it.
- `M` releases the mouse so you can select text, and says `mouse off` in the footer until `M` again.

### Changed

- The footer offers `m map`, and keeps it ahead of other hints while the map is hidden for want of room. With `glyphs = "ascii"`, which forces the map off, it leaves `m map` out.

## [0.7.0] - 2026-10-25

### Changed

- On wide terminals the timeline takes the spare width, with finer columns, so the names column stays at most about a third of the screen and the ruler labels space out to match.
- From 92 to 103 columns, and from 145 to 151, the names column is narrower than before (28 columns at 100, down from 52), so the city column and each place's local time no longer show there.
- Below 80 columns each row puts its name and time on one line and its lane on the full-width line under it, when the terminal is tall enough. A terminal too short for that keeps its lanes beside the names from 60 columns, and shows a plain list below 60.

## [0.6.0] - 2026-10-25

### Added

- `[[copy]]` tables in config add your own copy-as formats, replace a built-in one, or hide one with `hide = true`. The format marked `default = true` is the line in the times box, what `y` copies and what `teamzones say` prints.
- `teamzones config init` lists every built-in copy-as format, commented out, ready to edit.
- `[theme] range` sets how the range is marked (`columns`, `edges`, `lines`, `underline` or `focus`), and `R` cycles it for the session, naming the style in the footer.
- `llms.txt` tells a coding agent how to write a teamzones config, including how to gather colleagues' details from Slack, calendars and directory tools.

### Changed

- The copy-as dialog describes each format in plain words, naming the focused row for `t`, and leaves out a row whose line repeats an earlier one (its key still copies). When the formats do not fit two rows each, it lists one row per format.
- The copy-as dialog's footer shows only `? help` and `q quit`, since the dialog lists its own keys.

### Fixed

- `t` on a row in your own zone copies the time once, instead of repeating it in parentheses.

### Removed

- `TEAMZONES_DEBUG=1` and `F2`, replaced by `[theme] range` and `R`.

## [0.5.0] - 2026-10-24

### Added

- A trading profile can name its three levels with `labels = { closed = "...", open = "...", busy = "..." }`, such as `open = "trading hours"`, and those words show on its place rows, in the say line and in `teamzones status`.
- `c` opens a copy-as dialog that previews the exact line each format copies, with a short description under each, and `y` in it copies the plain line.
- With `TEAMZONES_DEBUG=1`, `F2` cycles five ways of marking the range so they can be compared, and the footer names the current one.

### Changed

- Place levels are now called `closed`, `open` and `busy` (they were `quiet`, `off-peak` and `peak`), and a group's `where` takes `open` or `closed`. The old `off-peak` and `quiet` still work.
- The say line names each place by its own state, such as `open in AU and US, closed in UK`, so a closed place is never called open.
- The who block shows each city in full when the row has room, cutting it only when the times need the space.
- The box at the bottom is titled `times for <group>` and holds only the line `y` copies. The focused row's details move to a dim line above it.
- The footer follows the mode: drawing a range shows a `range` badge and only the range keys, a committed range offers `esc clear range`, and `. now` shows whenever you are away from now. `1-9 group` moves to the `?` help.
- Under `NO_COLOR` or `scheme = "mono"`, the range is marked with edge lines instead of shading.

## [0.4.0] - 2026-10-24

### Added

- Running `teamzones` in a terminal opens the who and where screen: a slider over the viewed day, the who block with each person's day and hours, the where block with each place's trading load, and the say panel with the line `y` copies.
- Move through time with `h`/`l` (15 minutes), `H`/`L` (an hour) and `[`/`]` (a day), type a time or range with `t`, draw a range with `v`, nudge its edges with `{` `}` `<` `>`, and return to now with `.`.
- `n` and `N` jump to the next or previous slot that fits everyone the group asks about, keeping the range's length when it can and saying why when it cannot.
- `1`-`9` switch groups and `0` shows everyone; `space` marks or unmarks the focused row, `X` clears the marks to home, and `u` brings back marks a switch dropped.
- `c` copies a variant of the say line: with the date, with names, for the focused row, with place zones, without the clause, or for everyone.
- `?` lists the keys for where you are, then the keys that work everywhere, then what each glyph means.
- `NO_COLOR` turns colour off, and `glyphs = "ascii"` in `[theme]` draws everything in plain ASCII.

## [0.3.0] - 2026-10-24

### Added

- `teamzones say` prints the say line for a group and a time, such as `teamzones say team 3pm` or `teamzones say deploy next`, and copies it when run in a terminal (`--no-copy` skips that). `--zones`, `--names`, `--date` and `--no-clause` reshape the line.
- `teamzones status` shows where each person or place in a group is right now, and what comes next for each place.
- `teamzones groups` lists each group with its members and the question it asks.
- `teamzones config path`, `init`, `edit` and `check` find, create, open and validate the config file. A new file starts from a commented template that behaves exactly like no file.
- `teamzones here london --until 2026-11-01` records a trip in config without touching the rest of the file, and `teamzones here --clear` removes it.
- Without a config file, home is the system time zone.

## [0.2.0] - 2026-10-24

### Added

- `config.toml` is read and checked strictly, and any problem is reported with its line number.
- Cities resolve by name from a bundled GeoNames list, and an ambiguous name lists the candidates.
- Typed times such as `11-1pm`, `mon 3pm sast`, `26 oct` and `next` are understood, with zone abbreviations matched on the date typed.
- Fit search finds the slots where everyone a group asks about is available and every place it asks about is off-peak or quiet.
- The say line and status lines are written in the `ttz` style, ready for the CLI and TUI to print.

## [0.1.0] - 2026-10-24

### Added

- The `teamzones` binary, which so far prints its version with `teamzones --version`.
