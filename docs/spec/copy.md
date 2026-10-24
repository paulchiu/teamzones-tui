# Copy format

Seeded from the shared contract of the teamzones build spec (2026-10-24). Where a mockup and the text disagree, the text wins.

* **A zone entry** renders a time as `h[:mm]am|pm`, with minutes only when non-zero and `12am` for midnight. A range renders as `5-6pm` when both ends share a suffix and `11am-1pm` when they do not. The entry is followed by a space and the zone abbreviation from chrono-tz (`%Z`), used as is even if numeric.
* An entry gets a weekday prefix (`Sun 10pm-12am CDT`) when its local start date differs from the axis zone's local start date.
* **Order**: the axis zone first, then descending UTC offset at the range start, with ties broken by config order. Entries that render identically are merged (Melbourne and Brisbane outside DST). The separator is `" / "`.
* **Who contributes zones**: counted people (whether or not they qualify), home whenever the marks ask who, and any `+@city` clock. A places-only group contributes the axis zone alone. Places never add a zone except under `c z`.
* **Clause**: when the marks ask where (and `where` is not `context`), add `", "` (a comma and a space) followed by the states, grouped and joined as `A, B and C`.
  + Each place is named by its real state, never by the question's word, so a closed place in a group that asks `open` reads `closed in …`.
  + States the question counts come first: under `where = "open"`, `open in …` then `closed in …`, then `busy in …`. Under `where = "closed"`, `closed in …`, then `busy in …`, then `open in …`.
  + Each place uses its own profile's labels for these words, so places with different labels form separate groups, in config order.
  + Places within a group keep config order.
  + The state is the range state.
* **Variants**:
  + `c d` prefixes the axis zone's date, as in `"Mon 26 Oct, "` (the date, a comma and a space).
  + `c w` adds names after each zone entry, as in `(Paul)` or `(Priya, Sam)`, with home's name included.
  + `c t` is only the focused row's entry, followed by the axis entry in parentheses, with no clause.
  + `c z` replaces the clause with the counted places' zone entries.
  + `c s` drops the clause.
  + `c a` is the line for `0 all`.

The `c` menu applies one variant at a time, and previews each variant's line before you pick it (`y` in the menu is the plain line). CLI flags can combine, applied in the order date prefix, names, then clause handling.

## Copy golden table

Fixture config, now is `2026-10-23T22:04:00Z` (Sat 24 Oct 9:04am AEDT).

| Group, range (AEDT) | Keys | Clipboard |
| --- | --- | --- |
| team, Mon 26 Oct 5-6pm | `y` | `5-6pm AEDT / 4-5pm AEST / 8-9am SAST` |
| regions, Mon 2-4pm | `y` | `2-4pm AEDT, open in AU and US, closed in UK` |
| regions, Mon 2-4pm | `c d` | `Mon 26 Oct, 2-4pm AEDT, open in AU and US, closed in UK` |
| regions, Mon 2-4pm | `c z` | `2-4pm AEDT / 3-5am GMT / Sun 10pm-12am CDT` |
| deploy, Mon 5-5:30pm | `y` | `5-5:30pm AEDT / 4-4:30pm AEST / 8-8:30am SAST, open in AU, closed in UK` |
| deploy, Mon 5-5:30pm | `c w` | `5-5:30pm AEDT (Paul) / 4-4:30pm AEST (Priya) / 8-8:30am SAST (Thandi), open in AU, closed in UK` |
| deploy, Mon 5-5:30pm | `c s` | `5-5:30pm AEDT / 4-4:30pm AEST / 8-8:30am SAST` |
| team, `space` on UK, Mon 5-6pm | `y` | `5-6pm AEDT / 4-5pm AEST / 8-9am SAST, closed in UK` |
| regions, Mon 2-4pm, focus UK | `c t` | `3-5am GMT (2-4pm AEDT)` |
