# cli

This covers the headless commands: `say`, `status`, `groups`, `config` and `here`, the who-expressions they take, how typed times read, and the exit codes. See the [README](../README.md) for installing and a quick start, and [screen](screen.md) for the TUI that `teamzones` on its own opens.

The examples use the sample config from [configuration](configuration.md) on Saturday 24 October 2026 at 9:04am in Melbourne.

## `teamzones say [EXPR] [TIME] [flags]`

Prints the line for a group at a time and copies it. It copies only when stdout is a terminal, and says `📋 copied` on stderr when it does, so a script that captures stdout gets the line and nothing else. `--no-copy` turns the copy off on a terminal too.

```
$ teamzones say team 3pm
3pm AEDT / 2pm AEST / 6am SAST
$ teamzones say apps 3pm
3pm AEDT / 5am BST / Fri 11pm CDT, open in AU, closed in UK and US
$ teamzones say team 3pm --names
3pm AEDT (Paul) / 2pm AEST (Priya, Sam) / 6am SAST (Thandi, Johan)
```

The line starts with home's zone, then every other counted person's and place's zone by descending UTC offset, merging zones that show the same time. A group with places adds their states after a comma, so a place shows both its clock and whether it is open. With no `EXPR` it uses the first group, and with no `TIME` it means now.

| Flag | Does |
| --- | --- |
| `--zones` | Keeps each counted place's clock and drops its state, the old `ttz` shape. |
| `--names` | Adds who and which places are in each zone after it, home included, as in `(Paul, AU)` or `(UK)`. |
| `--date` | Starts the line with home's date, as in `Mon 26 Oct, `. |
| `--no-clause` | Drops the places altogether, clocks and states, leaving the people's times. |
| `--no-copy` | Prints without copying. |

The flags can go anywhere on the line and combine. A `[[copy]]` table in config can change what plain `say` prints; the flags then add to that.

`next` as the time finds the next slot where everyone counted qualifies, up to seven days ahead, and prints it dated, as long as the run lasts and at most an hour:

```
$ teamzones say deploy next
Mon 26 Oct, 5-5:30pm AEDT / 4-4:30pm AEST / 8-8:30am SAST / 6-6:30am GMT, open in AU, closed in UK
```

When nothing fits in seven days, it says which slot came closest on stderr and exits 1.

## Who-expressions

An expression starts with a group, an alias or `all`, then adds or takes away with `+` and `-`:

```
team +@uk         the team, asking about the UK too
regions -@us      the regions group without the US
team -sam         the team without Sam
deploy +au        deploy plus everyone in the au group
team +@tokyo      the team, with Tokyo's clock on the line but never counted
```

People and groups are named bare and places with `@`, so the group `au` and the place `AU` never get mixed up. `+@city` for a city that is not a configured place adds its clock to the line without counting it for anything. Mistakes say what you probably meant:

```
$ teamzones say uk now
uk is a place: did you mean @uk?
$ teamzones say regions -au 5pm
regions -au: none of au's people are in regions; did you mean -@au?
```

## Typed times

A time is a few words in any order, at most one of each kind:

| Kind | Examples |
| --- | --- |
| A clock time | `3pm`, `3:30pm`, `15:00`, `1500`, `noon`, `midnight` |
| A range | `3-5pm`, `11-1pm` (11am to 1pm), `3pm-5pm`, `15:00-17:00`, `10pm-2am` |
| A day | `today`, `tomorrow`, `mon` or `monday`, `26 oct`, `2026-10-26` |
| A zone | an abbreviation such as `sast`, a city, `@place` or an IANA name |
| Now or next | `now`, `next` |

Times are read in home's zone unless a zone is given, so `9am sast` is nine in the morning in Johannesburg. A bare `9-5` is an error, because it could mean either half of the day: write `9am-5pm` or `09:00-17:00`. A weekday means the next one from today, and a time with no day means today even when it has already passed, as `ttz` did. An abbreviation that means more than one offset (`IST`, `CST`) is an error listing the zones it could be.

## `teamzones status [EXPR | @place]`

One line per row, people first and home left out, padded into columns:

```
$ teamzones status regions
AU  open    Sat 9:04am AEDT  busy 11:30am-2:30pm
UK  closed  Fri 11:04pm BST  opens Sat 7am, next busy Sat 11:30am-2:30pm
US  open    Fri 5:04pm CDT   busy 6-10pm
$ teamzones status team
Priya   Sat 8:04am AEST   day off
Sam     Sat 8:04am AEST   day off
Thandi  Sat 12:04am SAST  night
Johan   Sat 12:04am SAST  night
```

A place line ends with what comes next, and a person line with their band.

## `teamzones groups`

```
$ teamzones groups
1 team: Priya, Sam, Thandi, Johan  asks who
2 au: Priya, Sam  asks who
3 regions (apps): @AU, @UK, @US  asks open
4 deploy: Priya, Thandi, @AU, @UK  asks who and open
```

The number is the key that selects the group in the TUI.

## `teamzones config path | init | edit | check`

`path` prints where the config is read from. `init` writes the commented template, and refuses if a file is already there. `edit` opens it in `$VISUAL`, then `$EDITOR`, then `vi`, writing the template first when there is no file. `check` validates it and prints the line and the reason for anything wrong.

## `teamzones here [CITY --until DATE | --clear]`

```
$ teamzones here London --until 2026-11-01
here London until 2026-11-01
$ teamzones here
here London until 2026-11-01
$ teamzones here --clear
home Melbourne
```

Setting travel writes `[travel]` into the config, and nothing else in the file changes. While it holds, the axis, typed times and the start of every line are London's, and home's own zone is still copied. It ends on its own once `until` is over in London; `--clear` ends it early. See [configuration](configuration.md#travel).

## Exit codes

| Code | Means |
| --- | --- |
| 0 | It worked. |
| 1 | Nothing fits, or there is nothing to say. |
| 2 | A usage mistake or a config error. |

`teamzones` on its own opens the TUI, and prints the help and exits 2 when stdin or stdout is not a terminal.
