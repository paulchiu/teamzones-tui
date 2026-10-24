#!/usr/bin/env python3
"""Generate crates/teamzones-core/data/cities.csv from GeoNames cities15000.

Run by hand and commit the output. Keeps every capital (feature code PPLC) and
every city of 1,000,000 people or more, plus Austin, Texas. The data is GeoNames
(https://www.geonames.org/), licensed CC BY 4.0.

Usage: python3 tools/cities.py path/to/cities15000.txt
(download and unzip https://download.geonames.org/export/dump/cities15000.zip first)
"""

import csv
import pathlib
import sys

OUT = pathlib.Path(__file__).resolve().parent.parent / "crates/teamzones-core/data/cities.csv"
MIN_POPULATION = 1_000_000
EXTRA = {("Austin", "US", "America/Chicago")}


def keep(row):
    name, country, tz = row[1], row[8], row[17]
    return row[7] == "PPLC" or int(row[14] or 0) >= MIN_POPULATION or (name, country, tz) in EXTRA


def main(source):
    with open(source, encoding="utf-8", newline="") as handle:
        rows = [row for row in csv.reader(handle, delimiter="\t", quoting=csv.QUOTE_NONE) if keep(row)]
    # Of several GeoNames entries with one name in one country, keep the most populous.
    best = {}
    for row in rows:
        key = (row[1], row[8])
        if key not in best or int(row[14] or 0) > int(best[key][14] or 0):
            best[key] = row
    lines = ["name,ascii,country,tz,lat,lon"]
    for row in sorted(best.values(), key=lambda r: (r[2].lower(), r[8])):
        fields = [row[1], row[2], row[8], row[17], row[4], row[5]]
        if any("," in field or '"' in field for field in fields):
            sys.exit(f"field needs quoting, which the loader does not support: {fields}")
        lines.append(",".join(fields))
    OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {len(lines) - 1} cities to {OUT}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(sys.argv[1])
