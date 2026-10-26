//! Folding a block's lines into the rows a short terminal leaves it.

use teamzones_core::question::Row;

/// One line of a block: a row, home's own city while travelling, the answer row, or the
/// `+N more` row standing for the lines folded under it. The clock list adds a city row holding
/// the people (and home's reference) who live there, and the `not in` rule over the rest.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    Row(Row),
    Reference,
    Answer,
    More(Vec<Entry>),
    City(Vec<Entry>),
    Outside,
}

impl Entry {
    /// The rows a `+N more` line or a city row stands for, or this line's own row.
    pub fn rows(&self) -> Vec<Row> {
        match self {
            Self::Row(row) => vec![*row],
            Self::More(folded) | Self::City(folded) => folded.iter().flat_map(Self::rows).collect(),
            Self::Reference | Self::Answer | Self::Outside => Vec::new(),
        }
    }

    /// The rows focus can sit on in this line: each of a city's people, or the first row a
    /// `+N more` line folds, which stands for it.
    pub fn focusable(&self) -> Vec<Row> {
        match self {
            Self::Row(_) | Self::City(_) => self.rows(),
            Self::More(_) => self.rows().into_iter().take(1).collect(),
            Self::Reference | Self::Answer | Self::Outside => Vec::new(),
        }
    }
}

/// `entries` cut to `room` screen lines, each entry taking `height` lines: they fill in order,
/// the answer row always shows, and the last line becomes `+N more` for whatever does not fit.
/// With room for every line nothing folds, and a folded `not in` rule is dropped.
pub fn fold(entries: Vec<Entry>, room: u16, height: impl Fn(&Entry) -> u16) -> Vec<Entry> {
    let total: u16 = entries.iter().map(&height).sum();
    if total <= room {
        return entries;
    }
    let answer = entries.iter().position(|entry| *entry == Entry::Answer);
    let mut lines: Vec<Entry> = entries
        .into_iter()
        .filter(|entry| *entry != Entry::Answer)
        .collect();
    let budget = room
        .saturating_sub(height(&Entry::Answer))
        .saturating_sub(height(&Entry::More(Vec::new())));
    let shown = lines
        .iter()
        .scan(0, |used, entry| {
            *used += height(entry);
            Some(*used)
        })
        .take_while(|used| *used <= budget)
        .count();
    let folded = lines
        .split_off(shown)
        .into_iter()
        .filter(|entry| *entry != Entry::Outside)
        .collect();
    lines.push(Entry::More(folded));
    match answer {
        Some(index) if index <= shown => lines.insert(index, Entry::Answer),
        Some(_) => lines.push(Entry::Answer),
        None => {}
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use teamzones_core::config::Config;

    const FIXTURE: &str = include_str!("../../../teamzones-core/tests/fixtures/sample.toml");

    /// The team group's who lines: home, four counted people, the answer row, two others.
    fn team() -> Vec<Entry> {
        let roster = Config::parse(FIXTURE).unwrap().roster;
        let person = |name| Entry::Row(Row::Person(roster.person_named(name).unwrap()));
        vec![
            person("Paul"),
            person("Priya"),
            person("Sam"),
            person("Thandi"),
            person("Johan"),
            Entry::Answer,
            person("Ellie"),
            person("Marcus"),
        ]
    }

    #[test]
    fn with_room_for_every_line_nothing_folds() {
        assert_eq!(fold(team(), 8, |_| 1), team());
    }

    #[test]
    fn short_of_room_the_last_line_folds_the_rest_and_the_answer_row_stays_below_the_counted() {
        let lines = team();
        let folded = fold(lines.clone(), 7, |_| 1);
        assert_eq!(
            folded,
            [
                &lines[..5],
                &[Entry::Answer, Entry::More(lines[6..].to_vec())]
            ]
            .concat()
        );
    }

    #[test]
    fn when_counted_rows_fold_the_answer_row_follows_the_fold_row() {
        let lines = team();
        let folded = fold(lines.clone(), 3, |_| 1);
        let more = [&lines[1..5], &lines[6..]].concat();
        assert_eq!(folded, [lines[0].clone(), Entry::More(more), Entry::Answer]);
    }
}
