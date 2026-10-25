# Headless CLI

Seeded from the shared contract of the teamzones build spec (2026-10-24). Where a mockup and the text disagree, the text wins.

| Command | Does |
| --- | --- |
| `teamzones` | Opens the TUI on the last used group (from state), or the first group. If stdin or stdout is not a TTY, it prints the help and exits 2. |
| `teamzones say [EXPR] [TIME] [--zones] [--names] [--date] [--no-clause] [--no-copy]` | Prints the say line (default TIME is now, default EXPR is the first group) and copies it (D10). `--zones`, `--names`, `--date` and `--no-clause` are `c z`, `c w`, `c d` and `c s`. A TIME of `next` prints the next fit, dated. |
| `teamzones status [EXPR \| @place]` | Prints one line per row of the expression (default: the first group), people first and home left out. A blank line separates the people from the places. The format is shown below. |
| `teamzones groups` | Prints one line per group: `1 team: Priya, Sam, Thandi, Johan  asks who`, or `3 regions (apps): @AU, @UK, @US  asks open`. |
| `teamzones config path \| init \| edit \| check` | Prints the path; writes the commented template (exits 2 if a file exists); opens `$VISUAL`, then `$EDITOR`, then `vi` (seeding the template first); or validates the file (exits 2 with the error). |
| `teamzones here [CITY --until DATE \| --clear]` | With no arguments, prints `here London until 2026-11-01` or `home Melbourne`. `--until` is required when setting. Otherwise it sets or clears `[travel]` through toml_edit. |
| `--now RFC3339` (hidden, global) | Fixes "now" for tests and demos. Every clock read goes through one `Clock` value. |

**Who-expression.** `EXPR := [group | alias | all] ( (+|-) (person | group | @place | @city) )*`. With no base, the first group is used. `±group` adds or removes all of that group's members. `+@city` adds a clock: its zone joins the say line, but it is never counted for a question. The tokens after `say` are read as EXPR while each one is a known group or alias, or starts with `+` or `-`. The rest is TIME. A token that neither parses as TIME nor names a group is an error with a did-you-mean, such as `uk is a place: did you mean @uk?`. A bare person name is an error too, such as `sam is a person: did you mean +sam?`. A modifier that changes nothing is also an error, such as `regions -au: none of au's people are in regions; did you mean -@au?`. Known long flags are accepted anywhere. Implement this with clap `trailing_var_arg` plus `allow_hyphen_values` on one positional and a pre-pass that pulls the known flags out.

**Exit codes:** 0 ok, 1 no fit or nothing to say, 2 usage or config error.

## CLI golden table

Fixture config, `--now 2026-10-23T22:04:00Z`, `--no-copy`.

| Command | stdout |
| --- | --- |
| `say team 3pm` | `3pm AEDT / 2pm AEST / 6am SAST` |
| `say au 3pm` | `3pm AEDT / 2pm AEST` |
| `say 3pm` | `3pm AEDT / 2pm AEST / 6am SAST` (first group is team) |
| `say apps 3pm` | `3pm AEDT / 5am BST / Fri 11pm CDT, open in AU, closed in UK and US` |
| `say apps 3pm --zones` | `3pm AEDT / 5am BST / Fri 11pm CDT` |
| `say regions 2-4pm mon` | `2-4pm AEDT / 3-5am GMT / Sun 10pm-12am CDT, open in AU and US, closed in UK` |
| `say regions -@us 5:30pm mon` | `5:30pm AEDT / 6:30am GMT, closed in UK, busy in AU` |
| `say regions -@au 5:30pm mon` | `5:30pm AEDT / 6:30am GMT / 1:30am CDT, closed in UK and US` |
| `say team +@uk 5pm mon` | `5pm AEDT / 4pm AEST / 8am SAST / 6am GMT, closed in UK` |
| `say deploy next` | `Mon 26 Oct, 5-5:30pm AEDT / 4-4:30pm AEST / 8-8:30am SAST / 6-6:30am GMT, open in AU, closed in UK` |
| `say regions next` | `Sat 24 Oct, 9:15-10am AEDT / Fri 11:15pm-12am BST / Fri 5:15-6pm CDT, open in AU and US, closed in UK` |
| `say team next` | `Mon 26 Oct, 5-6pm AEDT / 4-5pm AEST / 8-9am SAST` |
| `say uk now` | (stderr) `uk is a place: did you mean @uk?`, exit 2 |
| `say regions -au 5pm` | (stderr) `regions -au: none of au's people are in regions; did you mean -@au?`, exit 2 |

```
$ teamzones status regions --now 2026-10-23T22:04:00Z
AU  open    Sat 9:04am AEDT  busy 11:30am-2:30pm
UK  closed  Fri 11:04pm BST  opens Sat 7am, next busy Sat 11:30am-2:30pm
US  open    Fri 5:04pm CDT   busy 6-10pm
$ teamzones status team --now 2026-10-23T22:04:00Z
Priya   Sat 8:04am AEST   day off
Sam     Sat 8:04am AEST   day off
Thandi  Sat 12:04am SAST  night
Johan   Sat 12:04am SAST  night
```

Every column except the last is padded to its longest value, and columns are separated by two spaces. A place line ends with what comes next: the busy spell in progress or coming up today, or `opens …` while closed. A person line ends with the band, where the `off` band prints as `day off`.
