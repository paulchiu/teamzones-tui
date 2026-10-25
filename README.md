# teamzones

Who's available and where it's busy, on one time axis. A Rust TUI.

Picking a time for a call across Melbourne, Brisbane and Johannesburg is a sum you do in your head, and picking one for a deploy adds a second question: is any region we trade in at its lunch or dinner rush? `teamzones` answers both on one home-time axis. People (who you tell) are drawn by their hours and daylight, places (regions you operate in) by their trading load, and it finds the slots where everyone a group counts is free and every place it counts is quiet enough. Then it copies the answer as one line you can paste into Slack:

```
5-5:30pm AEDT / 4-4:30pm AEST / 8-8:30am SAST, open in AU, closed in UK
```

## Install

```sh
brew install paulchiu/tap/teamzones
```

Releases carry archives for macOS on Apple silicon and Intel, and for Linux on x86_64. From a checkout instead:

```sh
cargo install --path crates/teamzones
```

## Quick start

```sh
teamzones config init    # write a commented template to ~/.config/teamzones/config.toml
teamzones config edit    # fill in home, your colleagues, your regions and some groups
teamzones config check   # validate it, naming the line of any mistake
teamzones                # open the TUI
```

To have a coding agent write the config for you, point it at [llms.txt](llms.txt). It lists every table and field and tells the agent how to gather your colleagues' cities, zones and hours from tools such as Slack and your calendar.

## `say`

`teamzones say` prints a line for a group and copies it to the clipboard. With the sample config in [docs/configuration.md](docs/configuration.md), on a Saturday morning in Melbourne:

```
$ teamzones say team 3pm
3pm AEDT / 2pm AEST / 6am SAST
$ teamzones say apps 3pm
3pm AEDT, open in AU, closed in UK and US
$ teamzones say deploy next
Mon 26 Oct, 5-5:30pm AEDT / 4-4:30pm AEST / 8-8:30am SAST, open in AU, closed in UK
$ teamzones say team +@uk 5pm mon
5pm AEDT / 4pm AEST / 8am SAST, closed in UK
$ teamzones say regions 2-4pm mon --zones
2-4pm AEDT / 3-5am GMT / Sun 10pm-12am CDT
```

A place adds its state to the line, never its clock, unless you ask for `--zones`. `next` finds the next slot where the group's question is answered. Away from home, `teamzones here London --until 2026-11-01` moves the axis to London until that date is over there.

## The screen

Running `teamzones` with no arguments opens the TUI. This is it at 80x24, looking at the regions group at 9:04am on a Saturday:

```
teamzones   now Sat 24 Oct 9:04am AEDT                          ⌂ Melbourne AEDT
Sat 24 Oct AEDT                 12a · · · · 6a· · · · · 12p · · · · 6p· · · · ·
▸ 9:04am AEDT now               ─══════─────────══┃═─────────═══════────────────
 who   1 team  2 au  3 regions  4 deploy  0 all  +  ⋯  ─────────────────────────
⌂ Paul   Melbourne  9:04am AEDT Sat░░░░░░░░░░╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌░░░░░░░░
≡ who                        --  not asked: regions has no people
○ Priya  Brisbane   8:04am AEST ░░Sat░░░░░░░░╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌░░░░░░░░░░
○ Sam    Brisbane   8:04am AEST ░░Sat░░░░░░░░╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌░░░░░░░░░░
○ Thandi Johannesb.     12:04am ▓▓▚▚▚▚▚░░░░░░░░░░░Sat░░░░░░░░╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌
○ Johan  Johannesb.     12:04am ▓▓▚▚▚▚▚░░░░░░░░░░░Sat░░░░░░░░╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌
○ Ellie  London     Fri 11:04pm ▒▒▒▒▒▒▒▒░░░░░░░░░░░░Sat░░░░░░░░░░░░░▒▒▒▒▒▒▒▒▒▒▒▒
○ Marcus Austin      Fri 5:04pm ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░░░░░░░░░░Sat░░░░░░░░░░░░░
 where  asks open ──────────────────────────────────── ▁ closed  ▄ open  █ busy
◆ AU  9:04am AEDT          open Sat▁▁▁▁▁▁▁▁▁▁▁▄▄▄▄▄▄▄▄▄██████▄▄▄▄▄▄▄████████▄▄▁▁
◆ UK  Fri 11:04pm BST    closed █▄▄▄▄▄▄▄████████▄▄▁▁Sat▁▁▁▁▁▁▁▁▁▁▁▄▄▄▄▄▄▄▄▄█████
◆ US  Fri 5:04pm CDT       open ▄▄▄▄▄▄▄██████▄▄▄▄▄▄▄████████▄▄▁▁Sat▁▁▁▁▁▁▁▁▁▁▁▄▄
≡ open                      3/3 ⣇━━━━━━⣇⡄⡄⡄⡄⡄⣇⣇⣇━━━━⣇⣇⣇⡄⡄⡄⡄⡄⣇━━━━━━━⣇⣇⣇⣇⣇⣇⣇⡄⣇⣇⣇⣇
────────────────────────────────────────────────────────────────────────────────
▶ Paul  Melbourne  AEDT UTC+11  hours 9am-6pm Mon-Fri
╭ times for regions ─────────────────────────────────────────────────── y copy ╮
│ 9:04am AEDT, open in AU and US, closed in UK                                 │
╰───────────────────────────────────────────────────────── home zone, 3 places ╯

h/l time  v range  space mark  w save  tab groups  y copy  m map  ? help  q quit
```

`h` and `l` move through the day, `1` to `9` switch groups, `n` jumps to the next fit, `y` copies the line in the box and `?` lists every key. On a wide, tall terminal a world map with the night side sits above the ruler.

## Documentation

- [docs/configuration.md](docs/configuration.md): the config file, people, places, trading profiles, groups, travel and the theme.
- [docs/cli.md](docs/cli.md): every command, who-expressions, typed times and exit codes.
- [docs/screen.md](docs/screen.md): reading the screen, the keys, the mouse, folding on small terminals and editing from the TUI.
- [docs/releasing.md](docs/releasing.md): the label, tag and Homebrew pipeline, and building from source.
- [docs/spec/](docs/spec/) is the product contract the tests read, and [docs/decisions.md](docs/decisions.md) says why it is the way it is.

## Credits

City names, zones and coordinates come from [GeoNames](https://www.geonames.org/) (`cities15000`), used under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).

## Licence

MIT. See [LICENSE](LICENSE).
