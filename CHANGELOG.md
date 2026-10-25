# Changelog

This file documents notable changes to the project.

The format is based on [Keep a Changelog v1.1.0](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Marks you leave unsaved on a group, or on `0 all`, are kept in the state file when you quit and come back the next time you open that group.
- `places = "both"` in a `[[copy]]` table names each counted place's clock and its state, and is now the default.

### Changed

- A place you mark now adds its clock to the `times for` line, `y` and `teamzones say`, sorted in with the people's zones, and a place with a trading profile still adds its state after the comma. A place with no trading profile, which has no state, now shows up in the line at all.
- `--no-clause` (and `c s`) leaves the places out altogether, clocks and states, while `--zones` (and `c z`) keeps their clocks and drops their states.
- The times box summary counts the places' zones too, as in `4 zones for 5 people, 1 place` or `3 zones for 3 places`.
- `--names` (and `c w`) names the places in each zone too, after its people, as in `5-5:30pm AEDT (Paul, AU) / 4-4:30pm AEST (Priya) / 8-8:30am SAST (Thandi) / 6-6:30am GMT (UK)`.

### Fixed

- A `times for` line wider than its box now ends in `…` rather than being cut off without a mark.

## [0.13.0] - 2026-10-25

### Added

- `teamzones` opens on the group you last quit on, kept in `~/.local/state/teamzones/state.toml`.
- With no config file, the footer offers `E config` next to the `no config: E creates one` notice.

### Changed

- The times box now sits at the bottom with the notice row and the footer, and spare rows collect above it, so there is a blank row between the grid and the box whenever the terminal has room. With the map shown, the map gives up one row for it.

### Fixed

- On Linux, copying also writes the line to the terminal's clipboard (OSC 52), so terminals that support OSC 52 keep it when the system clipboard empties as `teamzones say` exits.

## [0.12.0] - 2026-10-25

### Added

- While `[travel]` is set, home moves to the travel city: the header, the axis, typed times and the start of every copied line use its zone, and your row there is marked `✈`, keeps your hours on its clock and still counts.
- While travelling, a dim `⌂` row keeps your home city's clock on screen. It never counts, but its zone is still copied, as in `3pm GMT / Tue 2am AEDT / Tue 1am AEST / 5pm SAST`.
- Travel ends by itself once `until` is over in the travel city, with nothing written to the config.
- On a terminal too short for every row, the blocks fold, the where block first, and the last row of a folded block reads `+N more`, saying how many of those rows are counted. The answer rows and the times box always show.
- `enter` on a `+N more` row, or a click on it, lists the rows it folds away, and `enter` or `esc` closes the list.
- Below 40 columns or 10 rows the screen says `terminal too small`, and `q` still quits.
- Releases for macOS (Apple silicon and Intel) and Linux x86_64 on GitHub, and `brew install paulchiu/tap/teamzones`.
- A quick start in the README, and user docs for the config, the CLI, the screen and releasing under `docs/`.

## [0.11.0] - 2026-10-25

### Changed

- When the map is off (hidden with `m`, by `[map] show = "never"`, for lack of room or under ascii glyphs), the rest of the screen moves up to sit under the header, and any spare rows gather above the footer. Pressing `m` again brings the map back above the blocks.

## [0.10.0] - 2026-10-25

### Added

- `z` opens map mode while the map is drawn, with a `map` badge and its keys in the footer. `esc` leaves it, and the view stays until `0` or a resize.
- In map mode, `h j k l` or the arrows pan the map, all the way round the world east and west.
- In map mode, `+` and `-` zoom the map in and out, out as far as the whole map, and `0` goes back to the default view.
- In map mode, the mouse wheel over the map pans it.
- The edge of the night side on the map is a smooth line, drawn in the dawn colour.

### Changed

- The map keeps its proportions at every size instead of stretching to fill its area. It fills the width and shows the band of latitudes around home and the counted rows, like the macOS Clock app, and when there is height to spare the whole map shows centred, with blank rows above and below.

## [0.9.0] - 2026-10-25

### Added

- `w` saves the marks into the active group, `+` saves them as a new group (offering a name such as `team_uk`), `=` cycles a group's `where`, and `x` closes the active group. Every change is written to `config.toml` keeping your comments and layout, and `u` within 10 seconds undoes it.
- A closed group disappears at once but is written only after 10 seconds or on quit, so `u` puts it back without touching the file.
- `tab` opens the groups matrix: people and places against every group, where `space` adds or removes a member, `R` renames a group (keeping the old name as an alias), `<` and `>` move it, `a` adds one and `x` closes one. The changes are written together when the matrix closes, and `u` steps back one change while it is open.
- `a` adds a person or a place through a short form, `e` edits the focused row (its name, city, hours, zone, trading profile or marker), and `d` deletes it from the config and every group, all with the same undo.
- `r` reloads the config, keeping the last good one and showing the error when the file does not load. `E` opens the config in `$VISUAL` or `$EDITOR`, creating it from the template when there is none, and reloads it when the editor exits.
- Without a config file, the TUI says `no config: E creates one`.
- Mouse: click the drift tag for a save or revert menu, click `+`, `⋯` or `asks open` for their keys, middle-click a group name to close it, double-click a row to edit it, and double-click the map to add someone or somewhere at that point. In the matrix, click a cell to toggle it, click a group to use it, drag it to move it, right-click to rename and middle-click to close.

### Changed

- The footer offers `w save` and `tab groups`, and the who strip ends in `+  ⋯`.
- An edit is never written over a config file changed outside teamzones since it was read; the notice says so and `r` reloads it.

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
