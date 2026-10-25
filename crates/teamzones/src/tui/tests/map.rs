//! The map: its labels, the night side and the sun, and when it shows.

use super::*;

/// Regions on Mon 26 Oct with a 2-4pm AEDT range, as the screen spec's second mockup draws it.
fn regions_monday() -> App {
    go_to(press(fixture(), &chars("3")), "26 oct 2-4pm")
}

/// The map's rows: everything between the header and the ruler.
fn map_text(buf: &Buffer) -> String {
    let ruler = y_of(buf, "Mon 26 Oct AEDT");
    (1..ruler)
        .map(|y| row(buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_map_labels_home_and_each_counted_place_with_its_state_over_the_range() {
    let map = map_text(&render(&regions_monday(), 120, 36));
    for label in [
        "UK closed 3-5am",
        "US open 10pm-12am",
        "AU open 2-4pm",
        "Melbourne 2-4pm",
    ] {
        assert!(map.contains(label), "no {label:?} in\n{map}");
    }
    insta::assert_snapshot!("regions_map_120x36", map);
}

/// The cell a latitude and longitude falls in on a 120 by 12 map one row below the header,
/// by plain arithmetic: 360 degrees over 120 columns, and 80N to 58S over 12 rows.
fn cell_of(lat: f64, lon: f64) -> (i32, i32) {
    let x = ((lon + 180.0) / 3.0).floor() as i32;
    let y = 1 + ((80.0 - lat) / 11.5).floor() as i32;
    (x, y)
}

#[test]
fn the_sun_sits_within_a_cell_of_the_subsolar_point_at_the_range_s_midpoint() {
    let buf = render(&regions_monday(), 120, 36);
    let suns: Vec<(i32, i32)> = (1..13)
        .flat_map(|y| (0..120).map(move |x| (x, y)))
        .filter(|(x, y)| buf[(*x, *y)].symbol() == "☼")
        .map(|(x, y)| (i32::from(x), i32::from(y)))
        .collect();
    // 3pm AEDT on Mon 26 Oct is 04:00 UTC: the sun is over 120E less the equation of time
    // (about 16 minutes, so 4 degrees), at a declination of about 12.6S.
    let (x, y) = cell_of(-12.6, 116.0);
    assert_eq!(suns.len(), 1, "{suns:?}");
    let (sx, sy) = suns[0];
    assert!(
        (sx - x).abs() <= 1 && (sy - y).abs() <= 1,
        "sun at {:?}, expected near {:?}",
        suns[0],
        (x, y)
    );
}
