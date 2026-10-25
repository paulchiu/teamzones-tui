# configuration

This covers the config file: home, people, places, trading profiles, groups, travel, the map, the theme and copy formats. See the [README](../README.md) for installing and a quick start, and [docs/spec/config.md](spec/config.md) for the contract the parser is held to.

TOML at `~/.config/teamzones/config.toml` (or `$XDG_CONFIG_HOME/teamzones/config.toml`), and `TEAMZONES_CONFIG` points it anywhere else. `teamzones config path` prints which one is in use.

The file is optional. Without one, home is your system time zone with no hours and there is nobody else, and the TUI says `no config: E creates one`. `teamzones config init` writes a template with every key commented out, which behaves exactly like no file until you uncomment something. Once any table is uncommented, `[home]` is required.

Parsing is strict. An unknown key, a bad range or a name used twice is an error naming the line it is on, because a typo that silently does nothing is worse than one that says so. `teamzones config check` runs the same check, and `r` in the TUI keeps the last good config when the file has a mistake in it.

## A full example

This is the sample config the tests use. Paul is home, everyone else is made up, and so are the trading hours.

```toml
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

## Home and people

`[home]` is you. Home's zone is the axis everything is drawn on, typed times are read in it, and every copied line starts with it. Each `[[person]]` is someone you tell.

- `city` is looked up in the bundled list of capitals and cities of a million people or more, ignoring case. A name two countries share is an error listing both. For a city the list lacks, give `label`, `tz` (an IANA name) and `at = [lat, lon]` instead, as Marcus does above.
- `hours` is a range such as `"9am-5pm"`, or the name of a `kind = "hours"` profile that several people share. `days` defaults to `"Mon-Fri"`, and takes a range (`Mon-Fri`, `Fri-Mon`), a list (`Mon,Wed,Fri`) or one day. A person's own `days` wins over their profile's.
- Someone without `hours` counts as reachable in daylight, like Ellie.

Day and night come from local sunrise and sunset. `[bands] source = "clock"` uses a fixed 6am to 6pm instead.

## Places and trading profiles

A `[[place]]` is a region you operate in. Nobody lives in it: it has one zone, an optional `trading` profile and a map position. To split a region across zones, add two places (`US-E`, `US-W`).

A `kind = "trading"` profile has `open` hours and `peak` ranges on the place's own clock. At any moment a place is `closed` outside its open hours, `busy` inside a peak and `open` otherwise. `[[profile.on]]` entries change that on some days, field by field, so one with only `peak` keeps the usual open hours, and `closed = true` closes the place all day. When two entries cover the same day, the one covering fewer days wins.

`labels` renames the three levels wherever that profile's places appear, which suits a support desk or an office better than hospitality words:

```toml
labels = { closed = "after hours", open = "trading hours", busy = "rush" }
```

A place with no `trading` is just a clock: it is drawn with day and night and never counts.

## Groups

A `[[group]]` is a named list of `people` and `places`, numbered 1 to 9 in file order, with optional `aliases`. Home belongs to every group that lists people. What a group asks follows from what it lists:

- people only: is everyone in hours (or in daylight)?
- places only: is every place quiet enough? Home is only the axis here.
- both: both at once, as the deploy group asks.

`where` says what quiet enough means: `"open"` (the default) counts a place that is not busy, `"closed"` only one that is closed, and `"context"` shows the places without counting them. The older `"off-peak"` and `"quiet"` still load as `"open"` and `"closed"`.

Groups, aliases and people share one namespace, ignoring case, so a group and a person cannot have the same name. Places have their own, written `@UK` in expressions. Names cannot contain `+`, `,`, `@` or spaces, start with `-`, or be `all`.

The TUI writes groups back for you: `w` saves the marks into the group, `+` makes a new one and `x` closes one. It rewrites only the lines it changes, so your comments and layout survive.

## Travel

```toml
[travel]
here  = "London"
until = 2026-11-01
```

While travel holds, London takes over home's jobs: the axis, typed times and the first entry on every line are London's, and your own row moves there and keeps your hours on London's clock. Your home city stays on screen as a dim reference row and its zone is still copied, so the team at home can read the line:

```
$ teamzones say team 3pm
3pm GMT / Tue 2am AEDT / Tue 1am AEST / 5pm SAST
```

Travel holds through the end of `until` in London and then stops by itself, with nothing written. `teamzones here` sets and clears it. It is never guessed from the system time zone.

## The map

`[map] show` is `"auto"` (the default: shown when the terminal has at least 100 columns and 12 spare rows), `"always"` (whenever there are at least two spare rows, since one stays over the times box) or `"never"`.

## Theme

```toml
[theme]
scheme = "dark"      # dark, light or mono
glyphs = "full"      # full or ascii
range  = "columns"   # columns, edges, lines, underline or focus
accent = "#0969da"   # any colour role by name, as #rrggbb
```

`dark` uses the terminal's own palette, `light` is fixed colours for a light background, and `mono` uses no colour; `NO_COLOR` forces `mono`. Every band and level has its own glyph, so nothing relies on colour alone. `glyphs = "ascii"` swaps every glyph for plain ASCII and turns the map off. The roles and the ASCII set are listed in [docs/spec/theme.md](spec/theme.md).

## Copy formats

`c` in the TUI lists every copy format with a preview of its line. The built-ins are `y` (the times), `d` (with the date), `w` (with names), `t` (the focused row's time first), `z` (each place's clock instead of its state), `s` (times only) and `a` (for everyone). A `[[copy]]` table with a built-in key replaces that format, `hide = true` removes one, and a new key adds one:

```toml
[[copy]]
key     = "y"
label   = "with the date"
date    = true
default = true
```

The one `default = true` format is the line in the times box, what `y` copies and what `teamzones say` prints. The template written by `config init` lists every built-in in full.

## For agents

[llms.txt](../llms.txt) teaches a coding agent to write this file, gathering your colleagues' cities, zones and hours from real sources. A test keeps it naming every field the parser accepts.
