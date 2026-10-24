# Config

Seeded from the shared contract of the teamzones build spec (2026-10-24). Where a mockup and the text disagree, the text wins.

## Data model and config

Config is optional. Without one, home is the system zone (the binary resolves it with the `iana-time-zone` crate and passes it into core), with no hours, and there are no people, places or groups. The TUI then shows a first-run notice: `no config: E creates one`. Parsing is strict: an unknown key or a bad value fails with the file, the line and the reason. A bad `r` reload keeps the last good config and shows the error in the notice row.

| Table | Keys | Rules |
| --- | --- | --- |
| `[bands]` | `source = "sun" \| "clock"` | Day and night come from local sunrise and sunset (`sun`, the default), or from a fixed 6am-6pm (`clock`). The same edges bound morning and evening for rows with hours. |
| `[home]` | `name`, `city` or (`label`, `tz`, `at`), `hours`, `days` | Required whenever any other table is present. |
| `[[person]]` | `name`, `city` or (`label`, `tz`, `at = [lat, lon]`), `hours` (a range such as `"9am-5pm"`, or a profile name), `days` (default `"Mon-Fri"`) | `city` resolves against the bundled list (see Cities). `label` plus `tz` plus `at` is for a city the list lacks. A person's own `days` overrides the profile's. |
| `[[place]]` | `name`, `tz`, `trading` (a profile name, optional), `at = [lat, lon]` | A place without `trading` is a plain clock row: it is drawn with day and night bands like a person without hours, and it never qualifies or adds to the clause. Its zone appears only under `c z`. |
| `[[profile]]` | `name`, `kind`. For hours: `hours`, `days`. For trading: `open`, `peak = [ranges]`, `[[profile.on]]` with `days` and any of `open`, `peak`, `closed = true` | For a local weekday, the `on` entry whose `days` covers the fewest days wins, and the later entry breaks a tie. It overrides field by field, so an entry with only `peak` keeps the base `open`. Profile ranges never cross midnight, but `"midnight"` is allowed as an end. |
| `[[group]]` | `name`, `aliases`, `people`, `places`, `where = "off-peak" \| "quiet" \| "context"` | Asks *who* if `people` is non-empty, and *where* if `places` is non-empty. `context` draws the places above the line but never counts them, and it is rejected at load on a group without people. |
| `[map]` | `show = "auto" \| "always" \| "never"` | `auto` applies the hiding rule in [Screen](screen.md). |
| `[travel]` | `here` (a city), `until` (a date) | Active through the end of `until` in `here`'s local date. Written by `teamzones here`. |
| `[theme]` | `scheme = "dark" \| "light" \| "mono"`, `glyphs = "full" \| "ascii"`, per-role hex overrides | See [Theme and glyphs](theme.md). |

**Days.** `days` is a range (`Mon-Fri`, or one that wraps such as `Fri-Mon`), a comma list (`Mon,Wed,Fri`) or one day. Dates in `[travel] until` are TOML local dates.

**Names.** Groups, aliases and people share one case-insensitive namespace. A clash fails at load and names both lines. Places have their own namespace, written `@name` in a who-expression (`team +@uk`), so the group `au` and the place `AU` never collide. Inside config lists, places are named plainly, because the key already says which kind. The characters `+ , @`, whitespace, a leading `-` and the name `all` are reserved. A `-` inside a name is fine (`jnb-office`, `US-E`).

**Cities.** Ship `crates/teamzones-core/data/cities.csv` (columns `name,ascii,country,tz,lat,lon`). Generate it from GeoNames `cities15000`, keeping every capital plus every city with population of 1,000,000 or more, and add Austin explicitly. Credit GeoNames (CC BY 4.0) in the README. A lookup matches `name` or `ascii` case-insensitively, and an ambiguous name is an error listing `name, country` candidates. `@city` works in headless expressions only, and falls back to this list after configured places.

**The sample config** is the [golden fixture](#golden-fixture). Ship it as `crates/teamzones-core/tests/fixtures/sample.toml`.

## Files, state and lifecycle

* Config: `$XDG_CONFIG_HOME/teamzones/config.toml`, falling back to `~/.config/teamzones/config.toml` (use `etcetera`, as blubat does). `TEAMZONES_CONFIG` overrides it.
* State: `~/.local/state/teamzones/state.toml`, holding the last group, the map override and the mouse capture setting. It is written atomically (temp file plus rename).
* Write-back: `toml_edit` rewrites only `[[group]]`, `[travel]` and the `[[person]]`/`[[place]]` entries the app adds or edits. Every write re-parses the result before replacing the file and is atomic. Model it on blubat's `save_dashboard` ([GitHub](https://github.com/paulchiu/blubat/blob/8b4b0d5f32dcff25193abb8c8ba0a5234b914253/blubat/src/config.rs#L67-L130)).
* Terminal: restore every enabled mode (raw mode, alternate screen, mouse capture) on quit, on editor handoff and in the panic hook. `ratatui::run()` installs a restore hook in 0.30.
* Redraw only on input, resize, a notice timeout, or the minute rolling over while viewing now. There is no animation.

## Golden fixture

The fixture config, the fixed instant and the facts every golden string depends on. The `[home]` person is Paul himself. Everyone else is fictional, and so are the trading profiles.

```toml
# crates/teamzones-core/tests/fixtures/sample.toml
[bands]
source = "sun"

[home]
name  = "Paul"
city  = "Melbourne"
hours = "9am-6pm"
days  = "Mon-Fri"

[[person]]
name  = "Priya"
city  = "Brisbane"
hours = "9am-5pm"

[[person]]
name  = "Sam"
city  = "Brisbane"
hours = "9am-5pm"

[[person]]
name  = "Thandi"
city  = "Johannesburg"
hours = "jnb-office"

[[person]]
name  = "Johan"
city  = "Johannesburg"
hours = "jnb-office"

[[person]]
name  = "Ellie"
city  = "London"

[[person]]
name  = "Marcus"
label = "Austin"
tz    = "America/Chicago"
at    = [30.27, -97.74]

[[place]]
name    = "AU"
tz      = "Australia/Melbourne"
trading = "venue"
at      = [-24.0, 134.0]

[[place]]
name    = "UK"
tz      = "Europe/London"
trading = "venue"
at      = [54.5, -2.5]

[[place]]
name    = "US"
tz      = "America/Chicago"
trading = "venue"
at      = [39.5, -96.0]

[[profile]]
name  = "jnb-office"
kind  = "hours"
hours = "8am-4pm"
days  = "Mon-Fri"

[[profile]]
name  = "venue"
kind  = "trading"
open  = "7am-11pm"
peak  = ["11:30am-2pm", "5:30-9pm"]
[[profile.on]]
days  = "Fri-Sat"
peak  = ["11:30am-2:30pm", "6-10pm"]

[[group]]
name   = "team"
people = ["Priya", "Sam", "Thandi", "Johan"]

[[group]]
name   = "au"
people = ["Priya", "Sam"]

[[group]]
name    = "regions"
aliases = ["apps"]
places  = ["AU", "UK", "US"]

[[group]]
name   = "deploy"
people = ["Priya", "Thandi"]
places = ["AU", "UK"]

[theme]
scheme = "dark"
```

### Facts

| Fact | Value |
| --- | --- |
| Now | `2026-10-23T22:04:00Z` = Sat 24 Oct 9:04am AEDT |
| Offsets on Sat 24 Oct | AEDT +11, AEST +10, SAST +2, BST +1, CDT -5 |
| DST changes | London BST ends 25 Oct 2026 01:00 UTC (GMT +0 from then). US CDT ends 1 Nov 2026. So on Mon 26 Oct, London is GMT and Chicago is still CDT. |
| Team overlap, Mon 26 Oct | Paul 9am-6pm AEDT, Priya and Sam 10am-6pm AEDT, Thandi and Johan 5pm-1am AEDT, so all five are in hours 5-6pm AEDT. |
| Regions fits, Mon 26 Oct (AEDT) | 1-3:30am, 8-9:30am, 2-5:30pm, 9-10:30pm |
| Deploy fit, Mon 26 Oct | 5-5:30pm AEDT (who is full 5-6pm, and AU's dinner peak starts 5:30pm) |
| Regions next from now | Sat 9:15-10am AEDT. The first slot after now is 9:15, and the US Friday dinner peak starts 6pm CDT, which is 10am AEDT. |
