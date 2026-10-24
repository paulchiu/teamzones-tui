# Product

Seeded from the shared contract of the teamzones build spec (2026-10-24). Where a mockup and the text disagree, the text wins.

## Summary

Build `teamzones`, a single binary with a TUI and a headless CLI. It shows people (who you tell) by availability and places (regions we operate in) by trading load on one home-time axis. It finds slots that answer a group's question, and copies a `ttz`-style line such as `5-5:30pm AEDT / 4-4:30pm AEST / 8-8:30am SAST, open in AU and UK`.

## Scope

### In scope

* A new public repo `paulchiu/teamzones-tui` cloned at `~/dev-misc/teamzones-tui`, with a two-crate workspace (`teamzones-core` lib, `teamzones` bin) and the CI, label and release automation copied from repon and blubat.
* Everything in the shared contract: config, time semantics, copy, CLI, screen, keymap, theme, travel.
* Homebrew release through cargo-dist into `paulchiu/homebrew-tap`.

### Out of scope

* Editing `~/dev-misc/teamzones` (the old Python `tz` CLI) or `~/dev-misc/teamzones-pocs`. Both are read-only references.
* Changing Paul's `ttz` function in `~/.zsh/functions/to_timezone` or anything else tracked by yadm. Swapping the alias over is Paul's manual step after M6.
* Publishing to crates.io. The names `teamzones` and `teamzones-core` are free as of 2026-10-24, but reserving them is Paul's call.
* Real load data (an hourly CSV per region), calendar integration, public holidays, multiple zones per region, system time zone travel detection, and a `[keys]` override table.

## The shared contract

This is the product. Every milestone builds against this section, and where a mockup and this text disagree, the text wins. Mockups are real ratatui renders from the V3-B POC (`~/dev-misc/teamzones-pocs/concept-v3-b/src/main.rs`, read-only). The POC's `say_line`, `level`, `next_fit` and `status_line` are worth reading as a reference, but they are not code to copy.

The words used here are defined in [GLOSSARY.md](../../GLOSSARY.md). The contract is split across [config](config.md), [time](time.md), [copy](copy.md), [cli](cli.md), [screen](screen.md), [keys](keys.md) and [theme](theme.md).

## Groups and marks

* Selecting a group with `1-9` sets the marks to its members, plus home if it asks who. Time, range and focus stay the same. Unsaved drift is dropped with a notice, and `u` restores it. Pressing the active group's number again reverts the drift.
* `space` toggles the focused row's mark. The row crosses its block's line, focus follows it by stable id (`enum Row { Person(Id), Place(Id) }`), and the inferred question updates. `X` clears the marks to home only, which asks who.
* `w` writes the marked people to `people` and the marked places to `places`, in config order, plus a changed `where`. A new `places` key goes directly after `people`. The notice reads, for example, `team: +@UK saved`, and `u` within 10 s un-writes it. `+` saves the marks as a new group, using an inline name prompt in the strip prefilled from the expression, lowercased, with `@` dropped and parts joined by `_` (`team_uk`). Groups past the ninth get no number and are reached through the matrix.
* `x` closes the active group with no confirm. The group disappears at once, the notice reads `closed deploy: its people and places stay  u undo`, and config is written after 10 s or on quit. `0 all` cannot be closed. It marks every person and place, is read-only, and `w` on it acts as `+`.
* `=` cycles `where` through `open`, `closed` and `context`, skipping `context` when no people are marked. The change shows as drift until `w`.
* `tab` opens the membership matrix overlay. People and then places run down the side, and groups run across with the question each asks. `space` toggles a cell, and home is locked at `⌂`. The overlay's bottom lines preview the group's say line and the equivalent CLI call. Matrix changes, including a close with `x` inside it, are written when the overlay closes. Until then, `u` undoes within the overlay.
* `a` opens a picker for a new person (name, city, hours) or a new place (name, zone, trading profile, marker). `e` edits the focused row, and `d` deletes it from config and every group, with the same 10 s `u`. Text prompts swallow every key except `esc` and `enter`.
* `u` is a single level of undo covering the last drop, write, close or delete, and it expires 10 s after the action. Nothing ever asks for confirmation, and no key confirms on `y`.

```
teamzones   now Sat 24 Oct 9:04am AEDT   viewing Mon 26 Oct +2d                                         ⌂ Melbourne AEDT
⣀⡀   ⢀⣀⣀⣀⣀⡀⣀⣀   ⣀⣀⣠⣲⣶⣶⣾⣿⡾⣿⣾⣿⣿⣷⣶⣿⡶⣶⣿⡙⡛⡒⠓⠐⠠⣤⣀⡀       ⣀⣴⠿         ⠉⠚⠓⡛⣛⣊⣀⣀      ⣤⣤⡆⡆⠒⣰⢄⣤⣀⣠⡄⠤⠔⠘⠘⠓⠛⠙⠒⠴⠴⠄⠤⠠⠠⢤⣀⣀⢔⡶⠤⠤⢤⢄ ⣀⡀  ⢀⢀⡀⣀
⠓⠛⠻⠶⣾⣿⣶⣀⢀⣀⣄⣀⣀⣉⣉⡉  ⠉⠁⠉⠉⠉⠉⠉⠉⠉⠉⢉⡭⠛⠿⠾⢷⡿⠿⢤⣿⣿⡃ ⠈⠻⢥⢀⣠⠖⠒⠐⠉⠈⠘⠒⠶⠖⠂  ⣀⡀UK quiet 6-6:30am⠈⠁⠉⠁⠉⠉⠐⠋                      ⡀⡀⣀⣀⡀⢤⣩⣥⡉⣉⠤⠰⠿
     ⠚⠋⠉⠉⠁     ⠘⠓⠶⣤⡀         ⠉⠉⠉⠙⠴⠉ ⣀⣠⣤⣭⣵⣶⣄             ⠰⠟◆●⠶⠔⠚⠛⠛⠓⠚⠉⠁ ⣀⢀⣀   ⣀⣀                          ⠐⢒⡳⣶⡀  ⠸⠵⠚⠁
                  ⠸⢄       ●      ⣤⠖⠚⠉⠉⠉⠁               ⠰⣏⣉⣩⡖⣊⣻⣗⢿⡷⠦⣴⣶⣛⣛⣛⡛⠓ ⠙⢿⣿⠂                    ⢤⣶⢤⣴⢒⣊⣁⣾⠟⠃
                    ⠉⠻⢷⣤⣀  ⢠⠒⠒⠒⠒⣶⣯⡅                   ⢀⡤⠞⠉     ⠉⠉⠓⠚⠉⠉⠒⠱⣿⣄  ⠐⢶⣤⣤⣄⣀⡀⣀⡀      ⡀       ⣀⣨⡗⠈⠉⠛⠉⠁
      ⠈⠉⠛               ⠈⠓⠲⠬⠶⢾⣽⣭⡍⠙⠛⣛⣓⡒⠂               ⣽                 ⠻⢵⣆⣠⣤⠤⠒⠋  ⠈⠉⢧⡀⢠⠖⠊⠉⠉⠦⢆⣀⡀⢫⡟⠉⠁⢰⣷⣀
                  ╭ groups ─────────────────────────────────────────────────────────────────────────╮⣛⣂⢀
                  │                         1 team     2 au       3 regions  4 deploy   + new       │4-4:30pm⣶⡛⠲⢤⣤
⠂                 │ asks                      who        who        where      both                 │m⠉◆⠈⠓⠚⠉⠳⢤●   ⢤⡗  ⠐⠒
                  │ ⌂ Paul    Melbourne       ⌂          ⌂          ⌂          ⌂                    │:30pm⡀⌂ ⢀⡽      ⣀
                  │   Priya   Brisbane        ●          ●          ·          ●                    │     ⠉⠉⠿⠏     ⣠⣴⠾⠛⠃
                  │   Sam     Brisbane        ●          ●          ·          ·                    │
Mon 26 Oct AEDT   │   Thandi  Johannesburg    ●          ·          ·          ●                    │   ·   9p  ·   ·
▸ 5-5:30pm AEDT 30│   Johan   Johannesburg    ●          ·          ·          ·                    │───────────────────
 who   1 team  2 a│   Ellie   London          ·          ·          ·          ·                    │ng  ░ night  ╌ off
⌂ Paul    5-5:30pm│   Marcus  Austin          ·          ·          ·          ·                    │▚▚▚░░░░░░░░░░░░░░░░
● Priya   4-4:30pm│ places                                                                          │░░░░░░░░░░░░░░░░░░░
● Thandi  8-8:30am│   AU      venue profile   ·          ·          ◆          ◆                    │▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
≡ in hours        │   UK      venue profile   ·          ·          ◆          ◆                    │⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄
○ Sam     4-4:30pm│   US      venue profile   ·          ·          ◆          ·                    │░░░░░░░░░░░░░░░░░░░
○ Johan   8-8:30am│                                                                                 │▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
○ Ellie    6-6:30a│ deploy   5-5:30pm AEDT / 4-4:30pm AEST / 8-8:30am SAST, off-peak in AU and UK   │▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
○ Marcus   1-1:30a│ cli      teamzones say deploy 5-5:30pm mon                                      │░░░░░░░░░░░░░░░░░░▒
 where  asks off-p│ cursor   space adds the place US to deploy (its state, not its zone)            │▄ off-peak  █ peak
◆ AU           off╰──────────────────────────────────────────────── written to config.toml on close ╯███████▄▄▄▄▄▄▄▄▁▁▁▁
◆ UK              quiet ████▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████████▄▄▄▄▄▄▄▄▁▁▁▁Mon▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████
≡ off-peak          2/2 ⡇⡇⡇⡇━━━━━━━━━━━━━━⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇━━━━━━━━━━━━━━⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇━━━━━━━━━━━━━━⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇━━━━━━⡇⡇⡇⡇⡇⡇
◇ US              quiet ▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████████▄▄▄▄▄▄▄▄▁▁▁▁Mon▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▄▄▄▄
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
╭ say deploy ────────────────────────────────────────────────────────────────────────────────────── y copies this line ╮
│ 5-5:30pm AEDT / 4-4:30pm AEST / 8-8:30am SAST, off-peak in AU and UK   3 zones for 3 people, 2 places                │
│ ▶ US  Mon 1-1:30am quiet  opens 7am  next peak 11:30am-2pm  CDT UTC-5  America/Chicago  venue profile                │
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯

space member  enter use  y copy  a new  R rename  </> move  x close  u undo  ? help  esc done
```

`tab` opens the membership matrix (V2-C's group columns, V2-B's overlay): people, then places, down the side; groups across, each with the question it asks. Every cell is a plain yes or no, because nothing joins through a city. The cursor is on US in deploy; the bottom lines show deploy's clipboard line and the CLI call that prints it. The footer switches to the matrix's keys, where `x` closes the group under the cursor with the same undo.
