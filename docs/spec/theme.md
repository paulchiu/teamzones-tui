# Theme and glyphs

Seeded from the shared contract of the teamzones build spec (2026-10-24). Where a mockup and the text disagree, the text wins.

* **Semantic roles** (foreground only, apart from the selection, which covers the focused row and, under `range = "columns"` or `"edges"`, the range's columns or ruler): `text`, `dim`, `accent`, `marker`, `ok`, `warn`, `danger`, `day`, `night`, `dawn` (morning/evening, and the map's terminator line), `work`, `quiet`, `night_tint` (the map background), `selection`.
* **dark** uses ANSI-16 names so it follows the terminal palette: text `Reset`, accent `LightBlue`, marker `LightRed` (orange where truecolor is available, `#fb923c`), ok `LightGreen`, warn `LightYellow`, danger `LightRed`, dim `DarkGray`, day `Yellow`, night `Blue`, quiet `Blue`, dawn `LightMagenta`, work `Green`, night_tint `#161b33` (no tint without truecolor), selection reversed. **light** is explicit RGB, taken from blubat's LIGHT palette (accent `#0969da`, ok `#1a7f37`, warn `#9a6700`, danger `#cf222e`, text `#24292f`, dim `#6e7781`, selection `#ddf4ff`), with the rest chosen to match. Override keys in `[theme]` are the role names. **mono** uses Reset plus DarkGray. `NO_COLOR` forces mono. The selection is the only background: the focused row, and whatever the range style shades.
* **Range style** (`[theme] range`, a user setting, [D26](../decisions.md)): `columns` (the default) shades the range's columns in every lane, on the ruler and on the answer rows; `edges` shades the ruler only and draws bars just outside the range on every lane; `lines` draws edge lines through every lane and answer row; `underline` underlines in-range cells; `focus` dims every cell outside the range. `R` cycles the style for the session without writing config, and the footer names the new style for 10 seconds. Under `NO_COLOR` or `mono`, the styles that need colour (`columns`, `edges` and `focus`) fall back to `lines`.
* Colour never carries meaning alone: every band and level has its own glyph, and lane height also carries the load level.
* **ascii glyph set** (`glyphs = "ascii"`):
  + marks: `⌂`→`^`, `✈`→`>`, `●`→`*`, `○`→`o`, `◆`→`@`, `◇`→`-`;
  + person lanes: `░`→`.`, `▒`→`:`, `▚`→`+`, `▓`→`#`, `╌`→`,`;
  + place lanes: `▁`→`_`, `▄`→`=`, `█`→`X`;
  + answer rows: none `.`, partial `-`, all `=`;
  + track: `-`, fit `=`, partial `~`, cursor `|`, range ends `[ ]`;
  + range marking: `▕ ▏ │`→`|`;
  + borders and corners `+ - |` (including `╭╮╰╯`), `≡`→`=`, `⋯`→`...`, `▸ ▶`→`>`, `◂`→`<`, `·`→`.`, `━ ═`→`=`, `┃`→`|`, `┣ ┫`→`[ ]`;
  + the map is forced off.
