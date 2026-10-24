# Glossary

The project's words, used in these senses in the code as well as the docs.

| Term | Meaning |
| --- | --- |
| Home | The `[home]` person and city, drawn `⌂`. Home is the axis zone, the zone typed times are read in, and the first entry in every copy. Home belongs to every group. |
| Here | A dated travel override (`✈`). While it is active, it takes over home's axis, typed-time and copy-first roles, and home becomes a reference row. |
| Person | Someone you tell. Has a city (zone, lat, lon) and optional office hours. Drawn as a band lane. |
| Place | A region we operate in. Has a zone, an optional trading profile and a map marker. Drawn as a level lane. Nobody lives in a place. |
| Profile | A named weekly schedule on a local clock: `kind = "hours"` (office hours) or `kind = "trading"` (open hours and peaks). |
| Group | A named, ordered list of people and places, numbered 1-9 in file order. A group's question is inferred from what it lists. |
| Counted | Marked: drawn above its block's line and contributing to the say line. Counted rows are what a question tests. |
| Qualifies | A counted row meets its block's rule at an instant: a person is in hours (or in daylight, without hours), and a place is off-peak (or quiet). |
| Marks | The working set of counted rows. Selecting a group sets the marks to its members. `space` changes them, and the difference from the saved group is the drift. |
| Question | *who* (counted people reachable), *where* (no counted place busy), or *both*. |
| Fit | A 15-minute slot where every counted row in every asked block qualifies. `n`/`N` and `say … next` look for fits. |
| Say line | Line 1 of the say panel. It is always exactly what `y` copies. |
