# Screen

Seeded from the shared contract of the teamzones build spec (2026-10-24). Where a mockup and the text disagree, the text wins.

Mockup lines are right-trimmed: trailing spaces were dropped from every frame line, so a test comparing a frame with a mockup pads each mockup line back to the frame width (120 or 80 columns) first.

The layout runs top to bottom with no outer frame and a horizontal margin of 0:

* header (1 row)
* map (rest of the height, while it is drawn in place)
* slider: ruler and track (2)
* who block: top border holding the lens strip, every person, and the answer row (people + 2)
* where block: top border, every place, answer row, bottom border (places + 3)
* detail line (1)
* spare rows (one row while the map is drawn in place, else the rest of the height)
* times box (3)
* notice row (1)
* footer (1)

The fixture has 7 people (home included) and 3 places, so the fixed rows total 24. Block heights come from the whole roster, never from the active group. The times box, the notice row and the footer keep to the bottom, with the footer on the last row. The rows the fixed rows leave sit together, blank, between the detail line and the times box, so the box has room to breathe. When the map is drawn in place it takes all of them but one, and that one stays over the times box. Otherwise the ruler starts on the row under the header and every spare row sits over the times box. With no spare rows the detail line sits directly over the times box.

The mockups below predate the hands-on round after M3 ([D17 to D22](../decisions.md)): they still show the old level words (`quiet`, `off-peak`, `peak`), the `say` panel with the detail line inside it and the old footer. They also predate [D39](../decisions.md), so their maps are stretched to the area rather than kept in proportion. Where they disagree with this text, the text wins.

```
teamzones   now Sat 24 Oct 9:04am AEDT   viewing Mon 26 Oct +2d                                         ⌂ Melbourne AEDT
⣀⡀   ⢀⣀⣀⣀⣀⡀⣀⣀   ⣀⣀⣠⣲⣶⣶⣾⣿⡾⣿⣾⣿⣿⣷⣶⣿⡶⣶⣿⡙⡛⡒⠓⠐⠠⣤⣀⡀       ⣀⣴⠿         ⠉⠚⠓⡛⣛⣊⣀⣀      ⣤⣤⡆⡆⠒⣰⢄⣤⣀⣠⡄⠤⠔⠘⠘⠓⠛⠙⠒⠴⠴⠄⠤⠠⠠⢤⣀⣀⢔⡶⠤⠤⢤⢄ ⣀⡀  ⢀⢀⡀⣀
⠓⠛⠻⠶⣾⣿⣶⣀⢀⣀⣄⣀⣀⣉⣉⡉  ⠉⠁⠉⠉⠉⠉⠉⠉⠉⠉⢉⡭⠛⠿⠾⢷⡿⠿⢤⣿⣿⡃ ⠈⠻⢥⢀⣠⠖⠒⠐⠉⠈⠘⠒⠶⠖⠂  ⣀⡀ ⢠⣠⣂⠊⢡⡠⣖⣂⣀⠁⠓⠗⠛⠛⠋ ⠈⠁⠉⠁⠉⠉⠐⠋                      ⡀⡀⣀⣀⡀⢤⣩⣥⡉⣉⠤⠰⠿
     ⠚⠋⠉⠉⠁     ⠘⠓⠶⣤⡀         ⠉⠉⠉⠙⠴⠉ ⣀⣠⣤⣭⣵⣶⣄             ⠰⠟◇●⠶⠔⠚⠛⠛⠓⠚⠉⠁ ⣀⢀⣀   ⣀⣀                          ⠐⢒⡳⣶⡀  ⠸⠵⠚⠁
                  ⠸⢄       ●      ⣤⠖⠚⠉⠉⠉⠁               ⠰⣏⣉⣩⡖⣊⣻⣗⢿⡷⠦⣴⣶⣛⣛⣛⡛⠓ ⠙⢿⣿⠂                    ⢤⣶⢤⣴⢒⣊⣁⣾⠟⠃
                    ⠉⠻⢷⣤⣀  ⢠⠒⠒⠒⠒⣶⣯⡅                   ⢀⡤⠞⠉     ⠉⠉⠓⠚⠉⠉⠒⠱⣿⣄  ⠐⢶⣤⣤⣄⣀⡀⣀⡀      ⡀       ⣀⣨⡗⠈⠉⠛⠉⠁
      ⠈⠉⠛               ⠈⠓⠲⠬⠶⢾⣽⣭⡍⠙⠛⣛⣓⡒⠂               ⣽                 ⠻⢵⣆⣠⣤⠤⠒⠋  ⠈⠉⢧⡀⢠⠖⠊⠉⠉⠦⢆⣀⡀⢫⡟⠉⠁⢰⣷⣀
                               ⠉⠙⣻⠏ ⠁⠉⠉⠉⠓⠒⠢⣄⡀          ⠙⠒⠤⠶⠴⠒⠲⠤⡄          ⣉⡥⠊        ⠙⠙⠂   ⠰⢾⣷⣏⠉⣠⠤⢾⣿⣛⣛⣂⢀
                                ⠐⠫⡀        ⠈⠈⠉⠉⣒⡆             ⠈⠑⣆       ⠐⡏  ⡀          ☼    Brisbane 4-5pm ·2⣶⡛⠲⢤⣤
⠂                                 ⠈⠑⢲       ⢀⣀⡴⠃               ⠐⢅   ● Johannesburg 8-9am ·2      ⢀⡤⠤⠔⠋⠉◇⠈⠓⠚⠉⠳⢤●   ⢤⡗  ⠐⠒
                                   ⢀⡎   ⢀⣀⣠⠜⠁                   ⠈⠳⣄⣀⣀⡤⠞⠉  ⠈⠉               Melbourne 5-6pm⡀⌂ ⢀⡽      ⣀
                                  ⢀⡾ ⢀⡶⠓⠉⠁                                                                ⠉⠉⠿⠏     ⣠⣴⠾⠛⠃
                                  ⠈⠲⠶⠯⠅⠐⠒                                         ⠈⠁
Mon 26 Oct AEDT         12a ·   ·   3a  ·   ·   6a  ·   ·   9a  ·   ·   12p ·   ·   3p  ·   ·   6p  ·   ·   9p  ·   ·
▸ 5-6pm AEDT 1h        ◂────────────────────────────────────────────────────────────────────┣══┫────────────────────────
 who   1 team  2 au  3 regions  4 deploy  0 all  +  ⋯  ──────────── ▒ day  ▚ morning/evening  ▓ working  ░ night  ╌ off
⌂ Paul       5-6pm AEDT Mon░░░░░░░░░░░░░░░░░░░░░░░▚▚▚▚▚▚▚▚▚▚▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▚▚▚▚▚▚▚▚░░░░░░░░░░░░░░░░
● Priya      4-5pm AEST ░░░░Mon░░░░░░░░░░░░░░░░░░▚▚▚▚▚▚▚▚▚▚▚▚▚▚▚▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▚▚▚▚▚░░░░░░░░░░░░░░░░░░░
● Sam        4-5pm AEST ░░░░Mon░░░░░░░░░░░░░░░░░░▚▚▚▚▚▚▚▚▚▚▚▚▚▚▚▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▚▚▚▚▚░░░░░░░░░░░░░░░░░░░
● Thandi     8-9am SAST ╌╌╌╌╌╌╌╌╌╌╌╌╌╌░░░░░░░░░░░░░░░░░░░░░░Mon░░░░░░░░░░░░░░░░░░░▚▚▚▚▚▚▚▚▚▚▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
● Johan      8-9am SAST ╌╌╌╌╌╌╌╌╌╌╌╌╌╌░░░░░░░░░░░░░░░░░░░░░░Mon░░░░░░░░░░░░░░░░░░░▚▚▚▚▚▚▚▚▚▚▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
≡ in hours          5/5 ····································⡀⡀⡀⡀⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇━━━━⡆⡆⡆⡆⡆⡆⡆⡆⡆⡆⡆⡆⡆⡆⡆⡆⡆⡆⡆⡆⡆⡆⡆⡆
○ Ellie       6-7am GMT ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░░░░░░░░░░░░░░░░░░░░░░░░░░░░Mon░░░░░░░░░░░░░░░░░░░░░░░░▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
○ Marcus      1-2am CDT ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░░░░░░░░░░░░░░░░░░░░Mon░░░░░░░░░░░░░░░░░░░░░░░░░░░░▒
 where  not asked ───────────────────────────────────────────────────────────────────────── ▁ quiet  ▄ off-peak  █ peak
≡ where              --  not asked: team has no places (space on a place adds one)
◇ AU               peak Mon▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████████▄▄▄▄▄▄▄▄▁▁▁▁
◇ UK              quiet ████▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████████▄▄▄▄▄▄▄▄▁▁▁▁Mon▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████
◇ US              quiet ▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████████▄▄▄▄▄▄▄▄▁▁▁▁Mon▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▄▄▄▄
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
╭ say team ──────────────────────────────────────────────────────────────────────────────────────── y copies this line ╮
│ 5-6pm AEDT / 4-5pm AEST / 8-9am SAST   3 zones for 5 people                                                          │
│ ▶ Priya  Brisbane  AEST UTC+10  hours 9am-5pm Mon-Fri  groups team, au, deploy  Mon 26 Oct 4-5pm in hours            │
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯

h/l time  v range  n next fit  space mark  1-9 group  w save  tab groups  y copy  c copy as  m map  ? help  q quit
```

120x36, a real ratatui `TestBackend` render. Now is Sat 24 Oct 9:04am AEDT; the view is scrubbed to Mon 26 Oct, 5-6pm AEDT (London is on GMT by then). The team group is active and focus is on Priya. Team lists no places, so the where block is all `◇`, its answer row says "not asked", and the clipboard is the v2 line unchanged: `5-6pm AEDT / 4-5pm AEST / 8-9am SAST`.

* **Header**: the app name in bold accent, then `now <day date time zone>` (dim) and `viewing <day date> +Nd` when the view is not today. `⌂ City ZONE` (or `✈ City ZONE`) is right-aligned. Pieces drop whole from the right when there is no room.
* **Columns**: label width `L` plus track width `T`. From 80 columns the label sits beside the track, which takes a whole number of columns per hour: the fewest that make `T` at least twice `L` (`cols / 36`, rounded up), cut back one at a time while `L` would fall under 20. Then `T = 24 × columns per hour` and `L = cols - T`, so the label is never wider than the track and from 120 columns is at most half of it. The examples are `80 columns: 32 + 48` (30 minutes a column), `100 columns: 28 + 72` (20 minutes), `120 columns: 24 + 96` (15 minutes), `160 columns: 40 + 120`, `200 columns: 56 + 144`, `250 columns: 82 + 168` and `320 columns: 104 + 216`. Below 80 columns rows stack (see Folding), and the track is the full width. From 60 to 79 columns, when the terminal is too short to stack, the label sits beside the track by the same rule. The track always shows the viewed axis-zone day from 12am to 12am, split evenly across its columns, and column i is the same instant in every row, the ruler and the track. A column's state is the grid slot it starts in, and the track marks a column `═` or `╌` by the slots it overlaps.
* **Ruler**: the label reads `Mon 26 Oct AEDT`. The ruler shows hour labels (`12a`, `3a` … `9p`) every 1, 2, 3, 4, 6 or 12 hours, the fewest that keep labels at least 10 columns apart, with `·` on each other hour. So 15 minutes a column labels every 3 hours, 30 minutes every 6, and 250 columns (seven columns an hour) every 2.
* **Track**:
  + The label reads `▸ 5-6pm AEDT 1h`, or `▸ 9:04am AEDT now` when the cursor is at now.
  + Cells draw `─`, or `═` in ok colour when every slot in the cell fits, or `╌` when only some do.
  + The cursor with no range draws `┃`, and a range draws `┣` … `┫` over its cells.
  + When now is outside the viewed day, a `◂` sits in the last column of the track label, and clicking it returns to now.
  + The fit marks (`═`, `╌`) are drawn first, then the cursor or range glyphs over them.
* **Who block**:
  + The top border holds `  who  ` (padded with a space each side), then the lens strip `1 team  2 au  3 regions  4 deploy  0 all  +  ⋯`. The active group is accent, bold and underlined, followed by any drift in warn colour, such as `+@UK` or `-Sam`.
  + The people legend is right-aligned in the border and lists only the bands on screen.
  + Rows go home first, then counted people, then the `≡` answer row, then the uncounted people dimmed.
  + Rows within each section are ordered by descending UTC offset, then config order.
* **Row label**: the mark (`⌂ ● ○`), the name, and the person's local time for the range, right-aligned (`4-5pm AEST`). A weekday prefix is added when the date differs from the axis date. When `L ≥ 32`, a city column sits after the name. It is as wide as the longest city among the people, and cuts with a `.` mark only when that would leave a row too little room for its time without the zone. So at 80 columns regions at now shows `Johannesb.`, team on Monday shows `Johannesburg`, and at 140 columns every city is whole. When the label is short of room, drop the zone first, then the minutes.
* **Person lane**: `░` night, `▒` day, `▚` morning or evening, `▓` working, `╌` off. A 3-letter weekday tag (`Mon`) is overwritten into the lane at local midnight.
* **Where block**:
  + The top border holds `  where  ` (padded with a space each side) and `asks open`, `asks closed`, `context` or `not asked`, plus the load legend `▁ closed  ▄ open  █ busy`. When every place uses the same `labels`, the legend shows those words instead.
  + Rows go counted places (`◆`), the answer row, then the other places (`◇`) dimmed.
  + A place label is its range state word (its profile's label for the level), worst over the range, with `busy` in warn bold. At 80 columns the label also shows the place's local clock.
  + Lanes draw `▁` closed (night colour), `▄` open (dim), `█` busy (warn), with day tags as for people.
* **Answer row**:
  + The label reads `≡ <rule>` with the count right-aligned (`≡ in hours 5/5`).
  + Each cell shows `·` when no row counts, a braille fill by fraction (`⡀ ⡄ ⡆ ⡇ ⣇ ⣧ ⣷`, nearest seventh), or `━` in ok colour when all count.
  + When the block is not asked, the row reads `-- not asked: team has no places (space on a place adds one)`, or the people version of that text.
* **Say panel**: the detail line, then the copy box under it.
  + The copy box is a rounded block titled `times for <expression>` (`times for all` on the `0 all` lens), with `y copy` right-aligned in the title. It holds only the say line, exactly what `y` copies. A dim summary (`3 zones for 5 people, 2 places`) sits right-aligned in its bottom border.
  + The detail line, directly above the box, is a dim `▶` plus the focused row's detail. It is never copied. For a person: city, zone, offset, hours, groups and state. For a place: state, what comes next, zone, profile and groups. Pieces drop whole from the right when there is no room.
* **Notice row**: blank unless there is something to say. A notice starts with its key, two spaces, then the text, plus a clickable `u undo` where it applies. It clears on the next key or after 10 s, whichever comes first. Clearing a notice does not cancel undo: `u` stays live for 10 s after the action. The texts are:
  + `n  next fit for deploy: Mon 5-5:30pm AEDT, range shortened from 1h to 30m (AU busy from 5:30pm)`
  + `3  dropped team +@UK  u undo`
  + `w  team: +@UK saved  u undo`
  + `x  closed deploy: its people and places stay  u undo`
  + `d  deleted Ellie from config and 2 groups  u undo`
  + `r  config reloaded`, or `r  config error line 12: unknown key "hour", kept the last good config`
  + `y  copied`, or `y  clipboard unavailable: 5-6pm AEDT / …`
* **Copy-as dialog**: `c` opens a bordered `copy as` dialog over the grid, with `esc close` in its bottom border. Each format ([copy](copy.md#formats), the built-ins `y d w t z s a` plus any `[[copy]]` ones) gets two rows: its key and the exact line it would copy right now, then its label, dim, with the focused row's name added in parentheses for a `focused-first` format. When two rows per format do not fit, each format gets one row and the labels are left out. A format whose line is identical to an earlier row's is left out, so a group with no places shows no repeated rows, but its key still copies that line. A preview or label too wide for the dialog is cut with `…`, but the key always copies the whole line. A format's key copies and closes the dialog, and `esc` closes it without copying. The footer shows only `? help` and `q quit` while it is open, since the dialog lists its own keys.
* **Footer**: ASCII only, generated from the binding table as a priority ladder, and it follows the mode. Hints keep their display order, and when space runs out they drop from the lowest priority up. `? help` and `q quit` are pinned and never drop, and mode hints are the last of the rest to drop.
  + In normal mode, highest priority first: `esc clear range` (only with a committed range), `. now` (only when the cursor or range is anywhere other than now), `E config` (only while there is no config file, so `E` creates one), `y`, `space`, `h/l`, `m` (raised to just below `h/l` while the map is auto-hidden, and otherwise last), `tab`, `v`, `n`, `w`, `c`. The display order is `h/l . v n space w tab y c m E esc ? q`. `1-9 group` is not in the footer, because the who strip already numbers the groups, and it stays in the `?` overlay.
  + While drawing a range with `v`, the footer leads with a `range` badge and shows only the range keys: `h/l move end  H/L 1h  o swap end  v/enter done  esc cancel`, then `? help`. Every main key still works in this mode.
  + In map mode, the footer leads with a `map` badge and shows only map mode's keys: `h/j/k/l pan  +/- zoom  0 reset  esc done`, then `? help`. Every other key is swallowed, apart from `q` and `ctrl-c`. The mouse wheel over the map pans it (up and down north and south, sideways or with shift west and east), and the wheel elsewhere does nothing while map mode is on.
  + After `R` changes the range style, the footer names the new style (`range style: edges`) at its right for 10 seconds.
* **Map** (M4, reworked in FB4):
  + ratatui `Canvas` with `Map { resolution: High }` and Braille markers, cropped to latitudes -58 to 80. The map keeps its proportions: a Braille dot (a cell is two dots across and four down, and twice as tall as it is wide) spans the same number of degrees of longitude as of latitude, so it never stretches to fill its area.
  + The default view fills the area's width with the whole world (360 degrees), with 0E in the middle, and crops latitude to the band centred midway between the northernmost and southernmost of home and the counted rows' markers, held so it never shows beyond 80N or 58S. When the area is tall enough for every latitude, the whole map shows centred, with blank rows above and below. So at 120x36 (a 12-row map) the band is 72 degrees tall: regions shows about 44N to 28S, with AU and the US in view and the UK and Melbourne cropped.
  + The view is worked out when the app starts, on `0` in map mode and on every resize, and stays put otherwise, so switching groups never moves the map.
  + Map mode (`z`, while the map is drawn in place or as the overlay) moves the view: `h j k l` and the arrows pan by an eighth of the drawn columns or a quarter of the drawn rows, and longitude wraps around. `+` (or `=`) zooms in a step of √2, up to four times the width, and `-` zooms out as far as the whole map showing; when the zoomed-out world is narrower than the area it is centred, with blank columns at the sides. `0` resets to the default view, and `esc` leaves map mode, keeping the view. A resize that hides the map ends map mode.
  + After the canvas renders, paint the night side onto the buffer: dim the foreground and add the night background tint. Then draw the terminator, the night side's edge, as a Braille line at dot resolution in the `dawn` colour: every dark dot with a lit dot beside, above or below it. Its cells show only the line's dots, over the cell shading, so it reads by shape under `mono`; the ascii glyph set never draws the map, so it has no line either. Draw `☼` at the subsolar point. Blank rows and columns outside the world are left unpainted.
  + Markers: `⌂` home, `●` counted people, `◆` counted places. Uncounted people are dim `●` and uncounted places dim `◇`. Uncounted markers are drawn first, so a counted marker wins a shared cell. Markers, labels, the sun, the night side and its terminator move with the view, and anything outside the view is not drawn.
  + Labels are drawn only for home and counted rows: `City 4-5pm` for a person (with `·N` after it when N counted people share the city, as in `Brisbane 4-5pm ·2`), and `UK closed 3-5am` for a place, using its profile's label for the state. Try the right of the marker on its row first, then the row above, then the left. Drop the label whole if all three overlap another label or leave the map's area.
  + The night side, its terminator and `☼` use the range midpoint, or the cursor when there is no range.
* **Map hiding**:
  + The map shows only if there are at least 100 columns and at least 12 rows are left after the fixed rows. It is drawn one row shorter than that, leaving the row over the times box blank.
  + Recompute on resize only, never on a group switch. `m` flips the automatic choice.
  + Whenever the map is not drawn in place (hidden by `m`, by `[map] show = "never"`, by this rule, or under `glyphs = "ascii"`), the blocks move up under the header and the spare rows go between the detail line and the times box, as in the layout above. `m` bringing the map back pushes the blocks down again. A group switch never moves the blocks. `m`, a resize, a config reload or a roster edit (`a`, `d`) can.
  + When the map is hidden and there is no room, `m` opens it as an overlay (a centred `Clear` box) until `esc`. The overlay is drawn over the blocks and never moves them.
* **Folding** (M6), used when the fixed rows exceed the terminal even without the map:
  + Block heights are recomputed per resize, never per group switch. The where block shrinks first, to its borders, the answer row and `min(places, 3)` rows. Then the who block shrinks to whatever is left, with at least home, the answer row and one more row.
  + Inside a block, rows fill in their usual order. The last row becomes `○ +N more` or `◇ +N more` when rows remain, listing how many of them are counted (`○ +3 more, 1 counted`). `enter` on it opens an overlay listing the folded rows.
  + Answer rows and the say panel never fold.
  + Below 80 columns each who and where row stacks: its label (mark, name, city when it fits, time or state) on one line, and its lane on the full-width line under it. The ruler, the track and each answer row stack the same way, so the cursor and range line up over the lanes. Stacking needs two lines per row, so it applies only when the terminal is tall enough for every stacked row. Otherwise, from 60 columns the label sits beside the track as in Columns, and below 60 the screen is a plain list: the label and state per row, without lanes or track cells.
  + Below 40 columns or 10 rows it shows `terminal too small`, and `q` still quits. The plain list is one line per row: mark, name, then the time or state label.

```
teamzones   now Sat 24 Oct 9:04am AEDT   viewing Mon 26 Oct +2d                                         ⌂ Melbourne AEDT
⣀⡀   ⢀⣀⣀⣀⣀⡀⣀⣀   ⣀⣀⣠⣲⣶⣶⣾⣿⡾⣿⣾⣿⣿⣷⣶⣿⡶⣶⣿⡙⡛⡒⠓⠐⠠⣤⣀⡀       ⣀⣴⠿         ⠉⠚⠓⡛⣛⣊⣀⣀      ⣤⣤⡆⡆⠒⣰⢄⣤⣀⣠⡄⠤⠔⠘⠘⠓⠛⠙⠒⠴⠴⠄⠤⠠⠠⢤⣀⣀⢔⡶⠤⠤⢤⢄ ⣀⡀  ⢀⢀⡀⣀
⠓⠛⠻⠶⣾⣿⣶⣀⢀⣀⣄⣀⣀⣉⣉⡉  ⠉⠁⠉⠉⠉⠉⠉⠉⠉⠉⢉⡭⠛⠿⠾⢷⡿⠿⢤⣿⣿⡃ ⠈⠻⢥⢀⣠⠖⠒⠐⠉⠈⠘⠒⠶⠖⠂  ⣀⡀UK quiet 3-5am⠛⠋ ⠈⠁⠉⠁⠉⠉⠐⠋                      ⡀⡀⣀⣀⡀⢤⣩⣥⡉⣉⠤⠰⠿
     ⠚⠋⠉⠉⠁     ⠘⠓⠶⣤⡀         ⠉⠉⠉⠙⠴⠉ ⣀⣠⣤⣭⣵⣶⣄             ⠰⠟◆●⠶⠔⠚⠛⠛⠓⠚⠉⠁ ⣀⢀⣀   ⣀⣀                          ⠐⢒⡳⣶⡀  ⠸⠵⠚⠁
                  ⠸⢄       ◆ US off-peak 10pm-12am      ⠰⣏⣉⣩⡖⣊⣻⣗⢿⡷⠦⣴⣶⣛⣛⣛⡛⠓ ⠙⢿⣿⠂                    ⢤⣶⢤⣴⢒⣊⣁⣾⠟⠃
                    ⠉⠻⢷⣤⣀  ⢠⠒⠒⠒⠒⣶⣯⡅                   ⢀⡤⠞⠉     ⠉⠉⠓⠚⠉⠉⠒⠱⣿⣄  ⠐⢶⣤⣤⣄⣀⡀⣀⡀      ⡀       ⣀⣨⡗⠈⠉⠛⠉⠁
      ⠈⠉⠛               ⠈⠓⠲⠬⠶⢾⣽⣭⡍⠙⠛⣛⣓⡒⠂               ⣽                 ⠻⢵⣆⣠⣤⠤⠒⠋  ⠈⠉⢧⡀⢠⠖⠊⠉⠉⠦⢆⣀⡀⢫⡟⠉⠁⢰⣷⣀
                               ⠉⠙⣻⠏ ⠁⠉⠉⠉⠓⠒⠢⣄⡀          ⠙⠒⠤⠶⠴⠒⠲⠤⡄          ⣉⡥⠊        ⠙⠙⠂   ⠰⢾⣷⣏⠉⣠⠤⢾⣿⣛⣛⣂⢀
                                ⠐⠫⡀        ⠈⠈⠉⠉⣒⡆             ⠈⠑⣆       ⠐⡏  ⡀                ⠉⠺⠷⠿⠭⠾⣽⣿⡬☼⢋⣻⡻⠭⣵⠦⣶⡛⠲⢤⣤
⠂                                 ⠈⠑⢲       ⢀⣀⡴⠃               ⠐⢅   ●  ⢠⠖⠋⣰⢊⠟        AU off-peak 2-4pm⠉◆⠈⠓⠚⠉⠳⢤●   ⢤⡗  ⠐⠒
                                   ⢀⡎   ⢀⣀⣠⠜⠁                   ⠈⠳⣄⣀⣀⡤⠞⠉  ⠈⠉               Melbourne 2-4pm⡀⌂ ⢀⡽      ⣀
                                  ⢀⡾ ⢀⡶⠓⠉⠁                                                                ⠉⠉⠿⠏     ⣠⣴⠾⠛⠃
                                  ⠈⠲⠶⠯⠅⠐⠒                                         ⠈⠁
Mon 26 Oct AEDT         12a ·   ·   3a  ·   ·   6a  ·   ·   9a  ·   ·   12p ·   ·   3p  ·   ·   6p  ·   ·   9p  ·   ·
▸ 2-4pm AEDT 2h        ◂────══════════──────────────────══════──────────────────┣══════┫══════──────────────══════──────
 who   1 team  2 au  3 regions  4 deploy  0 all  +  ⋯  ──────────── ▒ day  ▚ morning/evening  ▓ working  ░ night  ╌ off
⌂ Paul       2-4pm AEDT Mon░░░░░░░░░░░░░░░░░░░░░░░▚▚▚▚▚▚▚▚▚▚▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▚▚▚▚▚▚▚▚░░░░░░░░░░░░░░░░
≡ who                --  not asked: regions has no people, so home is the axis only (space on a person adds one)
○ Priya      1-3pm AEST ░░░░Mon░░░░░░░░░░░░░░░░░░▚▚▚▚▚▚▚▚▚▚▚▚▚▚▚▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▚▚▚▚▚░░░░░░░░░░░░░░░░░░░
○ Sam        1-3pm AEST ░░░░Mon░░░░░░░░░░░░░░░░░░▚▚▚▚▚▚▚▚▚▚▚▚▚▚▚▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▚▚▚▚▚░░░░░░░░░░░░░░░░░░░
○ Thandi     5-7am SAST ╌╌╌╌╌╌╌╌╌╌╌╌╌╌░░░░░░░░░░░░░░░░░░░░░░Mon░░░░░░░░░░░░░░░░░░░▚▚▚▚▚▚▚▚▚▚▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
○ Johan      5-7am SAST ╌╌╌╌╌╌╌╌╌╌╌╌╌╌░░░░░░░░░░░░░░░░░░░░░░Mon░░░░░░░░░░░░░░░░░░░▚▚▚▚▚▚▚▚▚▚▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
○ Ellie       3-5am GMT ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░░░░░░░░░░░░░░░░░░░░░░░░░░░░Mon░░░░░░░░░░░░░░░░░░░░░░░░▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
○ Marcus  Sun 10pm-12am ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░░░░░░░░░░░░░░░░░░░░Mon░░░░░░░░░░░░░░░░░░░░░░░░░░░░▒
 where  asks off-peak ───────────────────────────────────────────────────────────────────── ▁ quiet  ▄ off-peak  █ peak
◆ AU           off-peak Mon▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████████▄▄▄▄▄▄▄▄▁▁▁▁
◆ UK              quiet ████▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████████▄▄▄▄▄▄▄▄▁▁▁▁Mon▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████
◆ US           off-peak ▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████████▄▄▄▄▄▄▄▄▁▁▁▁Mon▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▄▄▄▄
≡ off-peak          3/3 ⣇⣇⣇⣇━━━━━━━━━━⣇⣇⣇⣇⡄⡄⡄⡄⡄⡄⣇⣇⣇⣇⣇⣇⣇⣇━━━━━━⣇⣇⣇⣇⣇⣇⣇⣇⡄⡄⡄⡄⡄⡄⣇⣇⣇⣇━━━━━━━━━━━━━━⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇━━━━━━⣇⣇⣇⣇⣇⣇
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
╭ say regions ───────────────────────────────────────────────────────────────────────────────────── y copies this line ╮
│ 2-4pm AEDT, off-peak in AU, UK and US   home zone, 3 places                                                          │
│ ▶ UK  Mon 3-5am quiet  opens 7am  next peak 11:30am-2pm  GMT UTC+0  Europe/London  venue profile                     │
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯

h/l time  v range  n next fit  space mark  1-9 group  w save  tab groups  y copy  c copy as  m map  ? help  q quit
```

120x36, regions (alias `apps`), Mon 26 Oct with a 2-4pm AEDT range and focus on UK. All-off-peak windows on Monday (AEDT): 1-3:30am, 8-9:30am, 2-5:30pm and 9-10:30pm, shown both on the `≡ off-peak` row and as `═` on the track. At 2-4pm AU is in its afternoon lull, UK is closed (3-5am GMT) and US is winding down on Sunday night (10pm-12am CDT, open until 11pm). `y` copies `2-4pm AEDT, off-peak in AU, UK and US`.

```
Mon 26 Oct AEDT         12a ·   ·   3a  ·   ·   6a  ·   ·   9a  ·   ·   12p ·   ·   3p  ·   ·   6p  ·   ·   9p  ·   ·
▸ 5-5:30pm AEDT 30m    ◂────────────────────────────────────────────────────────────────────┣┫──────────────────────────
 who   1 team  2 au  3 regions  4 deploy  0 all  +  ⋯  ──────────── ▒ day  ▚ morning/evening  ▓ working  ░ night  ╌ off
⌂ Paul    5-5:30pm AEDT Mon░░░░░░░░░░░░░░░░░░░░░░░▚▚▚▚▚▚▚▚▚▚▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▚▚▚▚▚▚▚▚░░░░░░░░░░░░░░░░
● Priya   4-4:30pm AEST ░░░░Mon░░░░░░░░░░░░░░░░░░▚▚▚▚▚▚▚▚▚▚▚▚▚▚▚▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▚▚▚▚▚░░░░░░░░░░░░░░░░░░░
● Thandi  8-8:30am SAST ╌╌╌╌╌╌╌╌╌╌╌╌╌╌░░░░░░░░░░░░░░░░░░░░░░Mon░░░░░░░░░░░░░░░░░░░▚▚▚▚▚▚▚▚▚▚▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
≡ in hours          3/3 ····································⡄⡄⡄⡄⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇⣇━━━━⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄⡄
○ Sam     4-4:30pm AEST ░░░░Mon░░░░░░░░░░░░░░░░░░▚▚▚▚▚▚▚▚▚▚▚▚▚▚▚▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▚▚▚▚▚░░░░░░░░░░░░░░░░░░░
○ Johan   8-8:30am SAST ╌╌╌╌╌╌╌╌╌╌╌╌╌╌░░░░░░░░░░░░░░░░░░░░░░Mon░░░░░░░░░░░░░░░░░░░▚▚▚▚▚▚▚▚▚▚▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
○ Ellie    6-6:30am GMT ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░░░░░░░░░░░░░░░░░░░░░░░░░░░░Mon░░░░░░░░░░░░░░░░░░░░░░░░▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
○ Marcus   1-1:30am CDT ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░░░░░░░░░░░░░░░░░░░░Mon░░░░░░░░░░░░░░░░░░░░░░░░░░░░▒
 where  asks off-peak ───────────────────────────────────────────────────────────────────── ▁ quiet  ▄ off-peak  █ peak
◆ AU           off-peak Mon▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████████▄▄▄▄▄▄▄▄▁▁▁▁
◆ UK              quiet ████▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████████▄▄▄▄▄▄▄▄▁▁▁▁Mon▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████
≡ off-peak          2/2 ⡇⡇⡇⡇━━━━━━━━━━━━━━⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇━━━━━━━━━━━━━━⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇━━━━━━━━━━━━━━⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇⡇━━━━━━⡇⡇⡇⡇⡇⡇
◇ US              quiet ▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████▄▄▄▄▄▄▄▄▄▄▄▄▄▄██████████████▄▄▄▄▄▄▄▄▁▁▁▁Mon▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▄▄▄▄
────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
╭ say deploy ────────────────────────────────────────────────────────────────────────────────────── y copies this line ╮
│ 5-5:30pm AEDT / 4-4:30pm AEST / 8-8:30am SAST, off-peak in AU and UK   3 zones for 3 people, 2 places                │
│ ▶ AU  Mon 5-5:30pm off-peak  peak 5:30-9pm  AEDT UTC+11  Australia/Melbourne  venue profile  groups regions, deploy  │
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
 n  next fit for deploy: Mon 5-5:30pm AEDT, range shortened from 1h to 30m (AU peak from 5:30pm)
h/l time  v range  n next fit  space mark  1-9 group  w save  tab groups  y copy  c copy as  m map  ? help  q quit
```

Crop of a 120x36 render, deploy, after `n` from Saturday. Nobody is in hours over the weekend, and on Monday the who row is full only from 5pm to 6pm AEDT (Thandi starts at 8am SAST, Priya stops at 5pm AEST). The where row is full then until AU's dinner peak starts at 5:30pm AEDT, so the track shows `═` for 5-5:30pm only. `n` lands there and shortens the 1h range to fit, saying why in the notice row while the footer stays put. US is not in deploy, so it sits below the line as `◇`. `y` copies `5-5:30pm AEDT / 4-4:30pm AEST / 8-8:30am SAST, off-peak in AU and UK`.

```
teamzones   now Sat 24 Oct 9:04am AEDT                          ⌂ Melbourne AEDT
Sat 24 Oct AEDT                 12a · · · · 6a· · · · · 12p · · · · 6p· · · · ·
▸ 9:04am AEDT now               ─══════─────────══┃═─────────═══════────────────
 who   1 team  2 au  3 regions  4 deploy  0 all  +  ⋯  ─────────────────────────
⌂ Paul   Melbourne  9:04am AEDT Sat░░░░░░░░░░╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌░░░░░░░░
≡ who                        --  not asked: regions has no people
○ Priya  Brisbane   8:04am AEST ░░Sat░░░░░░░░╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌░░░░░░░░░░
○ Sam    Brisbane   8:04am AEST ░░Sat░░░░░░░░╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌░░░░░░░░░░
○ Thandi Jo'burg   12:04am SAST ▓▓▚▚▚▚▚░░░░░░░░░░░Sat░░░░░░░░╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌
○ Johan  Jo'burg   12:04am SAST ▓▓▚▚▚▚▚░░░░░░░░░░░Sat░░░░░░░░╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌
○ Ellie  London     Fri 11:04pm ▒▒▒▒▒▒▒▒░░░░░░░░░░░░Sat░░░░░░░░░░░░░▒▒▒▒▒▒▒▒▒▒▒▒
○ Marcus Austin      Fri 5:04pm ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒░░░░░░░░░░Sat░░░░░░░░░░░░░
 where  asks off-peak ──────────────────────────────────────────────────────────
◆ AU  9:04am AEDT      off-peak Sat▁▁▁▁▁▁▁▁▁▁▁▄▄▄▄▄▄▄▄▄██████▄▄▄▄▄▄▄████████▄▄▁▁
◆ UK  Fri 11:04pm BST     quiet █▄▄▄▄▄▄▄████████▄▄▁▁Sat▁▁▁▁▁▁▁▁▁▁▁▄▄▄▄▄▄▄▄▄█████
◆ US  Fri 5:04pm CDT   off-peak ▄▄▄▄▄▄▄██████▄▄▄▄▄▄▄████████▄▄▁▁Sat▁▁▁▁▁▁▁▁▁▁▁▄▄
≡ off-peak                  3/3 ⣇━━━━━━⣇⡄⡄⡄⡄⡄⣇⣇⣇━━━━⣇⣇⣇⡄⡄⡄⡄⡄⣇━━━━━━━⣇⣇⣇⣇⣇⣇⣇⡄⣇⣇⣇⣇
────────────────────────────────────────────────────────────────────────────────
╭ say regions ───────────────────────────────────────────── y copies this line ╮
│ 9:04am AEDT, off-peak in AU, UK and US   home zone, 3 places                 │
│ ▶ UK  Fri 11:04pm quiet  opens Sat 7am  next peak Sat 11:30am-2:30pm         │
╰──────────────────────────────────────────────────────────────────────────────╯

h/l time  space mark  1-9 group  tab groups  y copy  m map  ? help  q quit
```

80x24, regions at now (Sat 24 Oct 9:04am AEDT, the cursor on `┃`), so this is the "is it peak right now?" glance. The map is hidden and the grid is 48 cells at 30 minutes. The wider label gives place rows their local clock as well as their state: UK closed at 11pm BST on Friday, so it reads quiet; US is under an hour from Friday's 6pm peak. Person labels keep the weekday (`Fri 11:04pm`) and drop the zone first. The track's `═` runs show the next all-off-peak windows; at 30 minutes a cell, `═` means the whole cell fits and `╌` would mark one that fits for only part of it (none today, since every Saturday window starts and ends on the half hour). The blank row above the footer is the notice row. The footer drops `v`, `n`, `w` and `c` but keeps `m map`.
