# Theme and glyphs

Seeded from the shared contract of the teamzones build spec (2026-10-24). Where a mockup and the text disagree, the text wins.

* **Semantic roles** (foreground only, apart from the selection): `text`, `dim`, `accent`, `marker`, `ok`, `warn`, `danger`, `day`, `night`, `dawn` (morning/evening), `work`, `quiet`, `night_tint` (the map background), `selection`.
* **dark** uses ANSI-16 names so it follows the terminal palette: text `Reset`, accent `LightBlue`, marker `LightRed` (orange where truecolor is available, `#fb923c`), ok `LightGreen`, warn `LightYellow`, danger `LightRed`, dim `DarkGray`, day `Yellow`, night `Blue`, quiet `Blue`, dawn `LightMagenta`, work `Green`, night_tint `#161b33` (no tint without truecolor), selection reversed. **light** is explicit RGB, taken from blubat's LIGHT palette (accent `#0969da`, ok `#1a7f37`, warn `#9a6700`, danger `#cf222e`, text `#24292f`, dim `#6e7781`, selection `#ddf4ff`), with the rest chosen to match. Override keys in `[theme]` are the role names. **mono** uses Reset plus DarkGray. `NO_COLOR` forces mono. The selected row is the only background.
* Colour never carries meaning alone: every band and level has its own glyph, and lane height also carries the load level.
* **ascii glyph set** (`glyphs = "ascii"`):
  + marks: `⌂`→`^`, `✈`→`>`, `●`→`*`, `○`→`o`, `◆`→`@`, `◇`→`-`;
  + person lanes: `░`→`.`, `▒`→`:`, `▚`→`+`, `▓`→`#`, `╌`→`,`;
  + place lanes: `▁`→`_`, `▄`→`=`, `█`→`X`;
  + answer rows: none `.`, partial `-`, all `=`;
  + track: `-`, fit `=`, partial `~`, cursor `|`, range ends `[ ]`;
  + borders and corners `+ - |` (including `╭╮╰╯`), `≡`→`=`, `⋯`→`...`, `▸ ▶`→`>`, `◂`→`<`, `·`→`.`, `━ ═`→`=`, `┃`→`|`, `┣ ┫`→`[ ]`;
  + the map is forced off.
