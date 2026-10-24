# Keymap

Seeded from the shared contract of the teamzones build spec (2026-10-24). Where a mockup and the text disagree, the text wins.

One binding table generates the footer and the `?` overlay (context first, then global, then a glyph legend). No hint is a literal string, and unbuilt keys are never advertised.

| Key | Action | Mouse | Milestone |
| --- | --- | --- | --- |
| h l, ← → | Move the cursor or range (or the active end in visual mode) 15 min; crossing midnight changes the viewed day | Wheel over the grid, one notch per 15 min (down or right is later); drag the range's middle on the track | M3 / M4 |
| H L, shift-← → | Move 1 hour | Shift-wheel | M3 / M4 |
| [ ] | Previous or next day | Horizontal scroll | M3 / M4 |
| . | Back to now and clear the range | Click `◂` or the header's now | M3 / M4 |
| t | Type a time or range (`tue 4pm`, `9am sast`) | none | M3 |
| v, o | Visual range: anchor, swap active end; `v` or `enter` commits | Drag across the grid or ruler | M3 / M4 |
| { } and < > | `{`/`}` move the start edge 15 min earlier or later, and `<`/`>` do the same for the end edge | Drag `┣` or `┫` | M3 / M4 |
| n N | Next or previous fit (see Time semantics) | Click a `═` run: the range moves to its start, shortened to the run if needed | M3 / M4 |
| esc | Unwind one layer: overlay, prompt, visual mode (restoring the range), then range (collapsing it) | Click outside an overlay | M3 |
| j k, ↑ ↓, g G | Focus a row, through the who block then the where block; first, last | Click a row label | M3 / M4 |
| space, X | Toggle the focused row's mark; clear marks to home | Click a mark glyph | M3 / M4 |
| enter | Expand or fold a `+N more` row | Click it | M6 |
| 1-9, 0 | Switch group (the active one again reverts drift); all | Click a strip name | M3 / M4 |
| w, +, x, = | Save; new group; close group; cycle where | Click the drift tag, which opens a two-item menu (`w save`, `revert`); click `+`; middle-click a name; click `asks …` | M5 |
| u | Undo the last drop, write, close or delete (within 10 s) | Click `u undo` | M5 (M3 for the switch drop) |
| tab | Membership matrix | Click `⋯` | M5 |
| y | Copy the say line | Click `y copies this line` | M3 / M4 |
| c | Copy as: `d w t z s a` (letters local to the menu) | Click a variant | M3 |
| b | People lanes: hours, or plain day and night (view only) | Click the who legend | M3 / M4 |
| s | Sort menu within each block: `o` offset, `n` name | Click a block title | M3 / M4 |
| m | Flip the map's automatic choice (overlay when there is no room) | none | M4 |
| M | Release or recapture the mouse | none | M4 |
| a e d | Add, edit or delete a person or place | Double-click the map (add, with the clicked point), double-click a label (edit) | M5 |
| E, r | Open config in `$EDITOR` (suspending the TUI); reload config | none | M5 |
| ?, q, ctrl-c | Help; quit; quit (always, even in prompts) | none | M3 |

Matrix overlay keys: `h j k l` and arrows move, `space` toggles, `enter` uses the group and closes, `y` copies its line, `a` adds a group, `R` renames (the old name becomes an alias), `< >` move the column, `x` closes the group under the cursor, `u` undoes, `esc` finishes. Mouse in the matrix: click a cell to move there and toggle it, click a column name to use that group, drag a column name to reorder, right-click to rename, middle-click to close.
