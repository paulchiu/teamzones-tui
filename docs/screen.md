# screen

This covers the TUI that `teamzones` opens: what each part of the screen shows, the keys and the mouse, small terminals, and editing groups, people and places from it. See the [README](../README.md) for a picture of it, and [docs/spec/screen.md](spec/screen.md) and [docs/spec/keys.md](spec/keys.md) for the contract the tests hold it to.

## Reading it

From the top:

- **The header** shows now in home's zone, `viewing` and the day offset once you scrub away from today, and `⌂ Melbourne AEDT` on the right (`✈ London GMT` while travelling).
- **The map**, on a terminal with room for it, marks home, people and places and shades the night side for the time you are looking at.
- **The ruler and track** cover one day in home's zone from midnight to midnight. `┃` is the cursor, `┣ ┫` the ends of a range, and `═` marks the times where the group's question is answered (`╌` where it is for part of a cell).
- **The who block** lists home, then the people the group counts (`●`), then a `≡` answer row, then everyone else dimmed (`○`). Each lane reads `▓` working, `▚` morning or evening, `▒` day, `░` night and `╌` a day off. The answer row's `5/5` counts who qualifies over the range, and its braille cells fill as more of them do.
- **The where block** does the same for places (`◆` counted, `◇` not), with lanes of `▁` closed, `▄` open and `█` busy.
- **The times box** holds exactly the line `y` copies, titled with the group, and the dim `▶` line above it describes the focused row.
- **The notice row** says what the last key did, such as `y  copied` or `n  next fit for deploy: ...`, and clears on the next key or after ten seconds.
- **The footer** lists the keys that matter right now, dropping the least useful first when it runs out of room.

Switching groups never moves anything: the blocks are sized by everyone in the config, not by the group, so the ruler, the borders and the times box stay put.

While travelling, home's row moves to the travel city and is marked `✈`, and a dim `⌂` row under it keeps your home city's clock in view. It is never counted, but its zone is still copied.

## Keys

`?` lists every key for where you are, and `q` or `ctrl-c` quits from anywhere.

| Keys | Do |
| --- | --- |
| `h l`, `← →` | Move 15 minutes (`H L` an hour, `[ ]` a day) |
| `.` | Back to now, dropping any range |
| `t` | Type a time or range, such as `tue 4pm` or `9am sast` |
| `v`, `o` | Draw a range (`o` swaps the end that moves), then `v` or `enter` |
| `{ } < >` | Move the start or end of the range |
| `n N` | The next or previous slot where the question is answered |
| `j k`, `↑ ↓`, `g G` | Focus a row |
| `space`, `X` | Count or stop counting the focused row; count only home |
| `enter` | List the rows a `+N more` row folds away |
| `1`-`9`, `0` | Switch group, or everyone |
| `w`, `+`, `x`, `=` | Save the marks to the group; save them as a new group; close the group; cycle `where` |
| `u` | Undo the last drop, save, close or delete, within 10 seconds |
| `tab` | The membership matrix: people and places down the side, groups across |
| `y`, `c` | Copy the line; copy as another format, with a preview of each |
| `b`, `s` | Lanes by hours or plain day and night; sort by offset or name |
| `R` | Cycle how the range is marked, for this session |
| `m`, `z` | Show or hide the map; pan and zoom it |
| `M` | Release the mouse so you can select text, and take it back |
| `a`, `e`, `d` | Add, edit or delete a person or place |
| `E`, `r` | Edit the config in `$EDITOR`; reload it |
| `esc` | Back out one step: an overlay, a prompt, drawing a range, then the range |

## The mouse

The mouse is on from the start, because the track is meant to be scrubbed. The wheel moves through the day, dragging on the track draws a range, and dragging its ends or middle moves them. Clicking a row focuses it, clicking a mark counts it, clicking a group name switches to it and clicking a `═` run jumps there. Clicking `+N more` lists what it folds, and double-clicking a label edits that row. Every click does what its key does, so the mouse never knows something the keys do not. `M` hands the mouse back to the terminal for selecting text.

## Small terminals

The layout keeps the label column in proportion to the track as the terminal widens. Below 80 columns each row stacks its label over a full-width lane when the terminal is tall enough, and below 60 columns without that height it becomes a plain list of labels and states.

When the terminal is too short for every row, the blocks fold. The where block gives up rows first, down to three places, then the who block, down to home and one more. The last row of a folded block becomes `○ +5 more, 3 counted`, and `enter` on it (or a click) lists the rows it stands for. It stands for no single row, so `space`, `e` and `d` leave it alone. The answer rows and the times box never fold, and a group switch still moves nothing. Below 40 columns or 10 rows the screen only says `terminal too small`, and `q` still quits.

## Editing from the screen

Every change the TUI makes goes straight into the config file through a format-preserving writer, touching only the lines it changes, so comments and layout survive. It never writes over a file that was changed behind its back: the notice says so, and `r` reloads.

- `space` marks drift from the saved group, shown in the strip as `+@UK` or `-Sam`. `w` saves it, and switching groups drops it with an undo.
- `+` saves the marks as a new group, with a name you can change before `enter`.
- `x` closes the group at once on screen, and writes it out only after the ten-second undo window.
- `tab` opens the matrix, where `space` toggles membership, `R` renames a group (the old name becomes an alias) and `< >` reorder columns. Its changes are written together when it closes.
- `a` asks for a person or a place, then their name, city and hours, or name, zone, trading profile and position. `e` edits the focused row and `d` deletes it from the config and every group.
- `E` hands the terminal to your editor and reloads when it exits.
