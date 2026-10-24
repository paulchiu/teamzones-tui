# Time semantics

Seeded from the shared contract of the teamzones build spec (2026-10-24). Where a mockup and the text disagree, the text wins.

* **Grid.** Every evaluation uses 15-minute slots aligned to UTC quarter hours. All of the zones involved here are whole or half-hour offsets, so the slots are quarter hours locally too. A range is the half-open interval `[start, end)`, and a single time is the slot that starts there.
* **Person band at an instant**, in local time:
  + With hours, on a listed work day: `working` inside hours. Otherwise `morning` from sunrise to the start of work, `evening` from the end of work to sunset, and `night` the rest. When sunrise falls after the start of work there is no morning band, and likewise for evening. Under `source = "clock"`, sunrise and sunset are 6am and 6pm.
  + With hours, on a day off: `off` while the sun is up (or 6am-6pm under `bands = "clock"`), otherwise `night`.
  + Without hours: `day` or `night`.
  + The sun is up when its elevation at the slot's start is above -0.833°, using the NOAA approximation (the POC's `sun_up`). Ignoring the equation of time is fine.
* **A counted person qualifies** when their band is `working`, or `day` if they have no hours. The who answer label is `in hours` if every counted person has hours, `daytime` if none do, and `hours or day` if it is mixed.
* **Place level at an instant**: `peak` inside a peak range for that local weekday, `off-peak` inside open hours otherwise, and `quiet` outside open hours or when `closed = true`. The level over a range is the worst slot (peak, then off-peak, then quiet).
* **A counted place qualifies** when it is not at peak (`where = "off-peak"`) or when it is quiet (`where = "quiet"`). The where answer label is `off-peak` or `quiet`.
* **Home** is counted whenever the active marks ask *who*. When they ask only *where*, home is the axis only.
* **A fit** is a slot where every counted row in every asked block qualifies. A range fits when every slot in it fits. The answer row's `k/n` counts the counted rows that qualify over the whole current range (or the cursor's slot).
* **The cursor** sits on exact now when viewing now (labels show `9:04am`, and states use the slot containing now). Any move snaps it to the grid: `l` from 9:04 goes to 9:15, and `h` goes to 9:00.
* **Next fit** (`n`, `say … next`):
  + CLI: the search starts at the first grid slot strictly after now. That can be the tail of a run already in progress (regions gives 9:15). The CLI takes the run's start, with a length of the run capped at 1h.
  + TUI: the search starts at the first slot after the current range start. If that slot is in the same fitting run as the range start, it skips to the end of that run, so repeated `n` steps run by run. With no range, `n` uses a 1h range. The range keeps its length when the run is long enough, otherwise it shrinks to the run and the notice says why.
  + `N` searches backwards the same way and lands on the start of the previous run.
  + The search covers up to 7 days. If nothing fits, the notice reads `no full slot in 7 days, best k/n at <when>` (for example `Mon 5pm AEDT`), and the cursor moves to that slot. The best slot is the earliest one with the most qualifying counted rows across both blocks. The CLI prints the same text to stderr and exits 1.
* **Typed times** (the `t` prompt and CLI `TIME`) are tokens in any order, with at most one of each kind:
  + clock: `3pm`, `3:30pm`, `15:00`, `1500`, `noon`, `midnight`;
  + range: `3-5pm`, `11-1pm` (11am-1pm), `3pm-5pm`, `15:00-17:00`, `10pm-2am`;
  + day: `today`, `tomorrow`, `mon`…`sun` or full names, `26 oct`, `2026-10-26`;
  + zone: an abbreviation (`sast`), a city, `@place` or an IANA name;
  + `now` and `next`.
* **Parsing rules for typed times:**
  + A bare `9-5` or `15-17` is an error asking for am/pm or `HH:MM`. A lone `midnight` is 12am at the start of the day, and `26 oct` is its next occurrence on or after today.
  + In a range with one suffix, the suffix applies to both ends, unless that puts the end at or before the start. In that case the start takes the other suffix.
  + An end at or before the start after that rule means the range crosses midnight.
  + A weekday means the next date on or after today in the axis zone.
  + No day means today. A time that has already passed is not moved to tomorrow, which matches `ttz`.
  + Abbreviations resolve against the configured zones on that date first. Failing that, they resolve against every chrono-tz zone's abbreviation on that date. An abbreviation that maps to more than one UTC offset (`IST`, `CST`) is an error listing the candidate zones. There is no hand-written list.
  + The default zone is the axis zone (home, or here while travelling).
