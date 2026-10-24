# Changelog

This file documents notable changes to the project.

The format is based on [Keep a Changelog v1.1.0](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

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
