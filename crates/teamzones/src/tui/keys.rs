//! The one binding table: it feeds the footer, the help overlay and the dispatcher.

/// A key press, independent of the terminal library that read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Ctrl(char),
    Left,
    Right,
    Up,
    Down,
    ShiftLeft,
    ShiftRight,
    Enter,
    Esc,
    Backspace,
    Tab,
    Other,
}

impl Key {
    /// The key a crossterm event names; shifted letters arrive as their capitals.
    pub fn from_crossterm(event: crossterm::event::KeyEvent) -> Self {
        use crossterm::event::{KeyCode, KeyModifiers};
        let shift = event.modifiers.contains(KeyModifiers::SHIFT);
        match event.code {
            KeyCode::Char(c) if event.modifiers.contains(KeyModifiers::CONTROL) => Self::Ctrl(c),
            KeyCode::Char(c) => Self::Char(c),
            KeyCode::Left if shift => Self::ShiftLeft,
            KeyCode::Right if shift => Self::ShiftRight,
            KeyCode::Left => Self::Left,
            KeyCode::Right => Self::Right,
            KeyCode::Up => Self::Up,
            KeyCode::Down => Self::Down,
            KeyCode::Enter => Self::Enter,
            KeyCode::Esc => Self::Esc,
            KeyCode::Backspace => Self::Backspace,
            KeyCode::Tab => Self::Tab,
            _ => Self::Other,
        }
    }
}

/// Where a key is read: the main screen, a modal, or everywhere (`Global`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
    Main,
    /// Drawing a range with `v`: its own keys first, then the main ones.
    Range,
    Global,
    Prompt,
    CopyMenu,
    SortMenu,
    Help,
    /// The membership matrix overlay.
    Matrix,
    /// `a`'s choice of what to add.
    AddMenu,
    /// The drift tag's menu.
    DriftMenu,
    /// Map mode: the map's own pan, zoom and reset keys.
    Map,
}

/// How rows are ordered within each block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    Offset,
    Name,
}

/// What a key does once dispatched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Earlier(i64),
    Later(i64),
    PreviousDay,
    NextDay,
    Now,
    TypeTime,
    Visual,
    SwapEnd,
    Commit,
    /// `enter`: open or close the list of the rows a focused `+N more` line folds.
    Fold,
    StartEarlier,
    StartLater,
    EndEarlier,
    EndLater,
    NextFit,
    PreviousFit,
    Unwind,
    FocusUp,
    FocusDown,
    FocusFirst,
    FocusLast,
    ToggleMark,
    ClearMarks,
    /// `w`: write the marks into the active group.
    Save,
    NewGroup,
    CloseGroup,
    CycleWhere,
    Group(usize),
    All,
    Undo,
    Copy,
    CopyMenu,
    /// Copy the format with this key, if the config has one.
    CopyAs(char),
    Lanes,
    SortMenu,
    SortBy(Sort),
    Help,
    Quit,
    Type(char),
    Erase,
    Submit,
    CycleRangeStyle,
    FlipMap,
    /// `z`: enter map mode, while the map is drawn.
    MapMode,
    /// Pan the map this many steps east and south.
    Pan(i8, i8),
    /// Zoom the map this many steps in.
    Zoom(i8),
    /// Put the map back on its default view.
    ResetMap,
    ToggleMouse,
    /// `tab`: open the membership matrix.
    Matrix,
    /// Move the matrix cursor this many columns and rows.
    Cell(i8, i8),
    ToggleCell,
    UseGroup,
    CopyGroup,
    AddGroup,
    RenameGroup,
    MoveGroup(i8),
    CloseColumn,
    FinishMatrix,
    AddMenu,
    /// Open the picker on a new place when true, a new person when false.
    AddRow(bool),
    EditRow,
    DeleteRow,
    Reload,
    OpenEditor,
    /// Drop the drift, back to the active group's saved members.
    Revert,
}

/// A footer hint: the keys as the footer writes them, a short label, and when it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hint {
    pub keys: &'static str,
    pub label: &'static str,
    pub shown: Shown,
    /// The priority it takes instead while `m` opens a map the hiding rule found no room for.
    pub raised: Option<u8>,
}

/// When a hint earns its footer space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    Always,
    /// Only while a range is committed.
    WithRange,
    /// Only while the cursor or range is anywhere other than now.
    AwayFromNow,
    /// Only while `m` can draw the map.
    WithMap,
    /// Only while there is no config file, so `E` would create one.
    WithoutConfig,
}

/// One row of the binding table; the main and global rows mirror `docs/spec/keys.md` row for row.
#[derive(Debug)]
pub struct Binding {
    /// The keys as the spec's keymap table writes them.
    pub keys: &'static str,
    /// What the keys do, as the help overlay says it.
    pub action: &'static str,
    /// The spec's milestone column; empty for modal rows the spec does not list.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "only the keymap spec test reads it")
    )]
    pub milestone: &'static str,
    pub context: Context,
    /// The footer hints, which drop from the lowest priority up; `PINNED` never drops.
    pub hints: &'static [Hint],
    pub priority: u8,
    /// The keys that dispatch, and what each does; empty while the row is unbuilt.
    pub chords: &'static [(Key, Action)],
}

/// The milestones built so far: M3 to M6 and the hands-on feedback rounds after M3.
#[cfg(test)]
pub const BUILT: [&str; 6] = ["M3", "M4", "M5", "M6", "FB2", "FB4"];

/// A footer priority that never drops.
pub const PINNED: u8 = u8::MAX;

const fn hint(keys: &'static str, label: &'static str) -> Hint {
    Hint {
        keys,
        label,
        shown: Shown::Always,
        raised: None,
    }
}

const fn hint_when(keys: &'static str, label: &'static str, shown: Shown) -> Hint {
    Hint {
        keys,
        label,
        shown,
        raised: None,
    }
}

/// The footer priority of `h/l`, which `m` ranks just below while the map has no room.
const TIME: u8 = 7;

/// The footer priority of a mode hint: below the pinned ones, above everything else.
const MODE: u8 = 200;

const fn row(
    keys: &'static str,
    action: &'static str,
    milestone: &'static str,
    context: Context,
) -> Binding {
    Binding {
        keys,
        action,
        milestone,
        context,
        hints: &[],
        priority: 0,
        chords: &[],
    }
}

const GROUP_CHORDS: [(Key, Action); 10] = [
    (Key::Char('1'), Action::Group(0)),
    (Key::Char('2'), Action::Group(1)),
    (Key::Char('3'), Action::Group(2)),
    (Key::Char('4'), Action::Group(3)),
    (Key::Char('5'), Action::Group(4)),
    (Key::Char('6'), Action::Group(5)),
    (Key::Char('7'), Action::Group(6)),
    (Key::Char('8'), Action::Group(7)),
    (Key::Char('9'), Action::Group(8)),
    (Key::Char('0'), Action::All),
];

/// The binding table, in footer display order within each context.
pub const BINDINGS: &[Binding] = &[
    Binding {
        hints: &[hint("h/l", "time")],
        priority: TIME,
        chords: &[
            (Key::Char('h'), Action::Earlier(1)),
            (Key::Char('l'), Action::Later(1)),
            (Key::Left, Action::Earlier(1)),
            (Key::Right, Action::Later(1)),
        ],
        ..row(
            "h l, ← →",
            "Move the cursor or range (or the active end in visual mode) 15 min; crossing midnight changes the viewed day",
            "M3 / M4",
            Context::Main,
        )
    },
    Binding {
        chords: &[
            (Key::Char('H'), Action::Earlier(4)),
            (Key::Char('L'), Action::Later(4)),
            (Key::ShiftLeft, Action::Earlier(4)),
            (Key::ShiftRight, Action::Later(4)),
        ],
        ..row("H L, shift-← →", "Move 1 hour", "M3 / M4", Context::Main)
    },
    Binding {
        chords: &[
            (Key::Char('['), Action::PreviousDay),
            (Key::Char(']'), Action::NextDay),
        ],
        ..row("[ ]", "Previous or next day", "M3 / M4", Context::Main)
    },
    Binding {
        hints: &[hint_when(".", "now", Shown::AwayFromNow)],
        priority: MODE,
        chords: &[(Key::Char('.'), Action::Now)],
        ..row(
            ".",
            "Back to now and clear the range",
            "M3 / M4",
            Context::Main,
        )
    },
    Binding {
        chords: &[(Key::Char('t'), Action::TypeTime)],
        ..row(
            "t",
            "Type a time or range (`tue 4pm`, `9am sast`)",
            "M3",
            Context::Main,
        )
    },
    Binding {
        hints: &[hint("v", "range")],
        priority: 4,
        chords: &[
            (Key::Char('v'), Action::Visual),
            (Key::Char('o'), Action::SwapEnd),
        ],
        ..row(
            "v, o",
            "Visual range: anchor, swap active end; `v` or `enter` commits",
            "M3 / M4",
            Context::Main,
        )
    },
    Binding {
        chords: &[
            (Key::Char('{'), Action::StartEarlier),
            (Key::Char('}'), Action::StartLater),
            (Key::Char('<'), Action::EndEarlier),
            (Key::Char('>'), Action::EndLater),
        ],
        ..row(
            "{ } and < >",
            "`{`/`}` move the start edge 15 min earlier or later, and `<`/`>` do the same for the end edge",
            "M3 / M4",
            Context::Main,
        )
    },
    Binding {
        hints: &[hint("n", "next fit")],
        priority: 3,
        chords: &[
            (Key::Char('n'), Action::NextFit),
            (Key::Char('N'), Action::PreviousFit),
        ],
        ..row(
            "n N",
            "Next or previous fit (see Time semantics)",
            "M3 / M4",
            Context::Main,
        )
    },
    Binding {
        hints: &[hint_when("esc", "clear range", Shown::WithRange)],
        priority: MODE + 1,
        chords: &[(Key::Esc, Action::Unwind)],
        ..row(
            "esc",
            "Unwind one layer: overlay, prompt, visual mode (restoring the range), then range (collapsing it)",
            "M3",
            Context::Global,
        )
    },
    Binding {
        chords: &[
            (Key::Char('j'), Action::FocusDown),
            (Key::Char('k'), Action::FocusUp),
            (Key::Down, Action::FocusDown),
            (Key::Up, Action::FocusUp),
            (Key::Char('g'), Action::FocusFirst),
            (Key::Char('G'), Action::FocusLast),
        ],
        ..row(
            "j k, ↑ ↓, g G",
            "Focus a row, through the who block then the where block; first, last",
            "M3 / M4",
            Context::Main,
        )
    },
    Binding {
        hints: &[hint("space", "mark")],
        priority: 9,
        chords: &[
            (Key::Char(' '), Action::ToggleMark),
            (Key::Char('X'), Action::ClearMarks),
        ],
        ..row(
            "space, X",
            "Toggle the focused row's mark; clear marks to home",
            "M3 / M4",
            Context::Main,
        )
    },
    Binding {
        chords: &[(Key::Enter, Action::Fold)],
        ..row(
            "enter",
            "Expand or fold a `+N more` row",
            "M6",
            Context::Main,
        )
    },
    Binding {
        chords: &GROUP_CHORDS,
        ..row(
            "1-9, 0",
            "Switch group (the active one again reverts drift); all",
            "M3 / M4",
            Context::Main,
        )
    },
    Binding {
        hints: &[hint("w", "save")],
        priority: 2,
        chords: &[
            (Key::Char('w'), Action::Save),
            (Key::Char('+'), Action::NewGroup),
            (Key::Char('x'), Action::CloseGroup),
            (Key::Char('='), Action::CycleWhere),
        ],
        ..row(
            "w, +, x, =",
            "Save; new group; close group; cycle where",
            "M5",
            Context::Main,
        )
    },
    Binding {
        chords: &[(Key::Char('u'), Action::Undo)],
        ..row(
            "u",
            "Undo the last drop, write, close or delete (within 10 s)",
            "M5 (M3 for the switch drop)",
            Context::Main,
        )
    },
    Binding {
        hints: &[hint("tab", "groups")],
        priority: 5,
        chords: &[(Key::Tab, Action::Matrix)],
        ..row("tab", "Membership matrix", "M5", Context::Main)
    },
    Binding {
        hints: &[hint("y", "copy")],
        priority: 10,
        chords: &[(Key::Char('y'), Action::Copy)],
        ..row("y", "Copy the say line", "M3 / M4", Context::Main)
    },
    Binding {
        hints: &[hint("c", "copy as")],
        priority: 1,
        chords: &[(Key::Char('c'), Action::CopyMenu)],
        ..row(
            "c",
            "Copy as: `y d w t z s a` (letters local to the menu), previewing each line",
            "M3",
            Context::Main,
        )
    },
    Binding {
        chords: &[(Key::Char('b'), Action::Lanes)],
        ..row(
            "b",
            "People lanes: hours, or plain day and night (view only)",
            "M3 / M4",
            Context::Main,
        )
    },
    Binding {
        chords: &[(Key::Char('R'), Action::CycleRangeStyle)],
        ..row(
            "R",
            "Cycle the range-marking style for this session (`[theme] range` sets the one it opens with)",
            "FB2",
            Context::Main,
        )
    },
    Binding {
        chords: &[(Key::Char('s'), Action::SortMenu)],
        ..row(
            "s",
            "Sort menu within each block: `o` offset, `n` name",
            "M3 / M4",
            Context::Main,
        )
    },
    Binding {
        hints: &[Hint {
            raised: Some(TIME - 1),
            ..hint_when("m", "map", Shown::WithMap)
        }],
        chords: &[(Key::Char('m'), Action::FlipMap)],
        ..row(
            "m",
            "Flip the map's automatic choice (overlay when there is no room)",
            "M4",
            Context::Main,
        )
    },
    Binding {
        chords: &[(Key::Char('z'), Action::MapMode)],
        ..row(
            "z",
            "Map mode, while the map is drawn: pan, zoom and reset its view",
            "FB4",
            Context::Main,
        )
    },
    Binding {
        chords: &[(Key::Char('M'), Action::ToggleMouse)],
        ..row("M", "Release or recapture the mouse", "M4", Context::Main)
    },
    Binding {
        chords: &[
            (Key::Char('a'), Action::AddMenu),
            (Key::Char('e'), Action::EditRow),
            (Key::Char('d'), Action::DeleteRow),
        ],
        ..row(
            "a e d",
            "Add, edit or delete a person or place",
            "M5",
            Context::Main,
        )
    },
    Binding {
        hints: &[hint_when("E", "config", Shown::WithoutConfig)],
        priority: 20,
        chords: &[
            (Key::Char('E'), Action::OpenEditor),
            (Key::Char('r'), Action::Reload),
        ],
        ..row(
            "E, r",
            "Open config in `$EDITOR` (suspending the TUI); reload config",
            "M5",
            Context::Main,
        )
    },
    Binding {
        hints: &[hint("?", "help"), hint("q", "quit")],
        priority: PINNED,
        chords: &[
            (Key::Char('?'), Action::Help),
            (Key::Char('q'), Action::Quit),
            (Key::Ctrl('c'), Action::Quit),
        ],
        ..row(
            "?, q, ctrl-c",
            "Help; quit; quit (always, even in prompts)",
            "M3",
            Context::Global,
        )
    },
    Binding {
        hints: &[hint("h/l", "move end")],
        priority: MODE + 5,
        chords: &[
            (Key::Char('h'), Action::Earlier(1)),
            (Key::Char('l'), Action::Later(1)),
            (Key::Left, Action::Earlier(1)),
            (Key::Right, Action::Later(1)),
        ],
        ..row("h l, ← →", "Move the active end 15 min", "", Context::Range)
    },
    Binding {
        hints: &[hint("H/L", "1h")],
        priority: MODE + 1,
        chords: &[
            (Key::Char('H'), Action::Earlier(4)),
            (Key::Char('L'), Action::Later(4)),
            (Key::ShiftLeft, Action::Earlier(4)),
            (Key::ShiftRight, Action::Later(4)),
        ],
        ..row(
            "H L, shift-← →",
            "Move the active end 1 hour",
            "",
            Context::Range,
        )
    },
    Binding {
        hints: &[hint("o", "swap end")],
        priority: MODE + 2,
        chords: &[(Key::Char('o'), Action::SwapEnd)],
        ..row("o", "Swap the active end", "", Context::Range)
    },
    Binding {
        hints: &[hint("v/enter", "done")],
        priority: MODE + 4,
        chords: &[
            (Key::Char('v'), Action::Visual),
            (Key::Enter, Action::Commit),
        ],
        ..row("v, enter", "Commit the range", "", Context::Range)
    },
    Binding {
        hints: &[hint("esc", "cancel")],
        priority: MODE + 3,
        chords: &[(Key::Esc, Action::Unwind)],
        ..row(
            "esc",
            "Cancel, restoring the range before",
            "",
            Context::Range,
        )
    },
    Binding {
        hints: &[hint("?", "help")],
        priority: PINNED,
        chords: &[(Key::Char('?'), Action::Help)],
        ..row("?", "Help", "", Context::Range)
    },
    Binding {
        hints: &[hint("h/j/k/l", "pan")],
        priority: MODE + 4,
        chords: &[
            (Key::Char('h'), Action::Pan(-1, 0)),
            (Key::Char('l'), Action::Pan(1, 0)),
            (Key::Char('k'), Action::Pan(0, -1)),
            (Key::Char('j'), Action::Pan(0, 1)),
            (Key::Left, Action::Pan(-1, 0)),
            (Key::Right, Action::Pan(1, 0)),
            (Key::Up, Action::Pan(0, -1)),
            (Key::Down, Action::Pan(0, 1)),
        ],
        ..row(
            "h j k l, arrows",
            "Pan the map west, south, north or east; longitude wraps",
            "",
            Context::Map,
        )
    },
    Binding {
        hints: &[hint("+/-", "zoom")],
        priority: MODE + 3,
        chords: &[
            (Key::Char('+'), Action::Zoom(1)),
            (Key::Char('='), Action::Zoom(1)),
            (Key::Char('-'), Action::Zoom(-1)),
        ],
        ..row(
            "+ =, -",
            "Zoom in; zoom out, as far as the whole map",
            "",
            Context::Map,
        )
    },
    Binding {
        hints: &[hint("0", "reset")],
        priority: MODE + 2,
        chords: &[(Key::Char('0'), Action::ResetMap)],
        ..row(
            "0",
            "Back to the default view: the whole width, on home and the counted rows",
            "",
            Context::Map,
        )
    },
    Binding {
        hints: &[hint("esc", "done")],
        priority: MODE + 1,
        chords: &[(Key::Esc, Action::Unwind)],
        ..row("esc", "Leave map mode, keeping the view", "", Context::Map)
    },
    Binding {
        hints: &[hint("?", "help")],
        priority: PINNED,
        chords: &[(Key::Char('?'), Action::Help)],
        ..row("?", "Help", "", Context::Map)
    },
    Binding {
        hints: &[hint("enter", "go")],
        priority: PINNED,
        chords: &[(Key::Enter, Action::Submit)],
        ..row("enter", "Go to the typed time", "", Context::Prompt)
    },
    Binding {
        hints: &[hint("esc", "cancel")],
        priority: PINNED,
        chords: &[(Key::Esc, Action::Unwind), (Key::Backspace, Action::Erase)],
        ..row(
            "esc, backspace",
            "Cancel; erase a character",
            "",
            Context::Prompt,
        )
    },
    Binding {
        chords: &[
            (Key::Char('h'), Action::Cell(-1, 0)),
            (Key::Char('l'), Action::Cell(1, 0)),
            (Key::Char('k'), Action::Cell(0, -1)),
            (Key::Char('j'), Action::Cell(0, 1)),
            (Key::Left, Action::Cell(-1, 0)),
            (Key::Right, Action::Cell(1, 0)),
            (Key::Up, Action::Cell(0, -1)),
            (Key::Down, Action::Cell(0, 1)),
        ],
        ..row(
            "h j k l, arrows",
            "Move between groups and rows",
            "",
            Context::Matrix,
        )
    },
    Binding {
        hints: &[hint("space", "member")],
        priority: MODE + 9,
        chords: &[(Key::Char(' '), Action::ToggleCell)],
        ..row(
            "space",
            "Add the row to the group, or take it out",
            "",
            Context::Matrix,
        )
    },
    Binding {
        hints: &[hint("enter", "use")],
        priority: MODE + 8,
        chords: &[(Key::Enter, Action::UseGroup)],
        ..row("enter", "Use the group and close", "", Context::Matrix)
    },
    Binding {
        hints: &[hint("y", "copy")],
        priority: MODE + 7,
        chords: &[(Key::Char('y'), Action::CopyGroup)],
        ..row("y", "Copy the group's line", "", Context::Matrix)
    },
    Binding {
        hints: &[hint("a", "new")],
        priority: MODE + 4,
        chords: &[(Key::Char('a'), Action::AddGroup)],
        ..row("a", "Add a group", "", Context::Matrix)
    },
    Binding {
        hints: &[hint("R", "rename")],
        priority: MODE + 3,
        chords: &[(Key::Char('R'), Action::RenameGroup)],
        ..row(
            "R",
            "Rename the group, keeping the old name as an alias",
            "",
            Context::Matrix,
        )
    },
    Binding {
        hints: &[hint("</>", "move")],
        priority: MODE + 2,
        chords: &[
            (Key::Char('<'), Action::MoveGroup(-1)),
            (Key::Char('>'), Action::MoveGroup(1)),
        ],
        ..row("< >", "Move the group a column", "", Context::Matrix)
    },
    Binding {
        hints: &[hint("x", "close")],
        priority: MODE + 5,
        chords: &[(Key::Char('x'), Action::CloseColumn)],
        ..row("x", "Close the group", "", Context::Matrix)
    },
    Binding {
        hints: &[hint("u", "undo")],
        priority: MODE + 6,
        chords: &[(Key::Char('u'), Action::Undo)],
        ..row("u", "Undo the last change here", "", Context::Matrix)
    },
    Binding {
        hints: &[hint("?", "help"), hint("esc", "done")],
        priority: PINNED,
        chords: &[
            (Key::Char('?'), Action::Help),
            (Key::Esc, Action::FinishMatrix),
        ],
        ..row(
            "?, esc",
            "Help; close, writing the changes to config",
            "",
            Context::Matrix,
        )
    },
    Binding {
        hints: &[hint("p", "person"), hint("l", "place")],
        priority: 5,
        chords: &[
            (Key::Char('p'), Action::AddRow(false)),
            (Key::Char('l'), Action::AddRow(true)),
        ],
        ..row("p l", "Add a person; add a place", "", Context::AddMenu)
    },
    Binding {
        hints: &[hint("w", "save"), hint("r", "revert")],
        priority: 5,
        chords: &[
            (Key::Char('w'), Action::Save),
            (Key::Char('r'), Action::Revert),
        ],
        ..row("w r", "Save the drift; revert it", "", Context::DriftMenu)
    },
    Binding {
        hints: &[hint("o", "offset"), hint("n", "name")],
        priority: 5,
        chords: &[
            (Key::Char('o'), Action::SortBy(Sort::Offset)),
            (Key::Char('n'), Action::SortBy(Sort::Name)),
        ],
        ..row(
            "o n",
            "Sort by UTC offset; sort by name",
            "",
            Context::SortMenu,
        )
    },
];

/// The contexts a key falls through to after its own; a prompt takes every key but `ctrl-c` as
/// text, and range mode keeps every main key.
fn fallbacks(context: Context) -> &'static [Context] {
    match context {
        Context::Main
        | Context::Global
        | Context::CopyMenu
        | Context::SortMenu
        | Context::Help
        | Context::Matrix
        | Context::AddMenu
        | Context::DriftMenu
        | Context::Map => &[Context::Global],
        Context::Range => &[Context::Main, Context::Global],
        Context::Prompt => &[],
    }
}

/// Whether a context's footer adds the global hints; range mode names only its own keys.
fn hints_global(context: Context) -> bool {
    match context {
        Context::Main
        | Context::Global
        | Context::CopyMenu
        | Context::SortMenu
        | Context::Help
        | Context::AddMenu
        | Context::DriftMenu => true,
        Context::Range | Context::Prompt | Context::Matrix | Context::Map => false,
    }
}

fn chord_in(context: Context, key: Key) -> Option<Action> {
    BINDINGS
        .iter()
        .filter(|binding| binding.context == context)
        .flat_map(|binding| binding.chords.iter())
        .find(|(chord, _)| *chord == key)
        .map(|(_, action)| *action)
}

/// What `key` does in `context`, or `None` when the context swallows it.
pub fn action_for(context: Context, key: Key) -> Option<Action> {
    if key == Key::Ctrl('c') {
        return Some(Action::Quit);
    }
    chord_in(context, key)
        .or_else(|| {
            fallbacks(context)
                .iter()
                .find_map(|fallback| chord_in(*fallback, key))
        })
        .or_else(|| unbound(context, key))
}

/// What a key no binding claims does: a prompt types printable characters, and the rest swallow it.
fn unbound(context: Context, key: Key) -> Option<Action> {
    match context {
        Context::Prompt => {
            if let Key::Char(c) = key {
                Some(Action::Type(c))
            } else {
                None
            }
        }
        Context::Main
        | Context::Range
        | Context::Global
        | Context::CopyMenu
        | Context::SortMenu
        | Context::Help
        | Context::Matrix
        | Context::AddMenu
        | Context::DriftMenu
        | Context::Map => None,
    }
}

/// The hints a context's footer may show, in display order, each with its priority.
pub fn hints(context: Context) -> Vec<(Hint, u8)> {
    let own = BINDINGS.iter().filter(move |b| b.context == context);
    let global = BINDINGS
        .iter()
        .filter(move |b| b.context == Context::Global && hints_global(context));
    own.chain(global)
        .filter(|binding| !binding.chords.is_empty())
        .flat_map(|binding| binding.hints.iter().map(|hint| (*hint, binding.priority)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYS_SPEC: &str = include_str!("../../../../docs/spec/keys.md");

    /// The keymap table's rows as (key, action, milestone) cells.
    fn spec_rows() -> Vec<(String, String, String)> {
        KEYS_SPEC
            .lines()
            .skip_while(|line| !line.starts_with("| Key |"))
            .take_while(|line| line.starts_with('|'))
            .skip(2)
            .map(|line| {
                let cells: Vec<String> = line
                    .trim_matches('|')
                    .split(" | ")
                    .map(|cell| cell.trim().to_owned())
                    .collect();
                (cells[0].clone(), cells[1].clone(), cells[3].clone())
            })
            .collect()
    }

    fn spec_bindings() -> impl Iterator<Item = &'static Binding> {
        BINDINGS
            .iter()
            .filter(|b| matches!(b.context, Context::Main | Context::Global))
    }

    #[test]
    fn compiled_table_matches_the_keymap_spec_row_for_row() {
        let compiled: Vec<(String, String, String)> = spec_bindings()
            .map(|b| {
                (
                    b.keys.to_owned(),
                    b.action.to_owned(),
                    b.milestone.to_owned(),
                )
            })
            .collect();
        assert_eq!(compiled, spec_rows());
    }

    #[test]
    fn a_row_dispatches_only_when_its_milestone_is_built() {
        for binding in spec_bindings() {
            assert_eq!(
                !binding.chords.is_empty(),
                BUILT.iter().any(|built| binding.milestone.contains(built)),
                "{}",
                binding.keys
            );
        }
    }

    #[test]
    fn every_hinted_key_dispatches_in_its_context() {
        for binding in BINDINGS.iter().filter(|b| !b.hints.is_empty()) {
            for (key, action) in binding.chords {
                assert_eq!(
                    action_for(binding.context, *key),
                    Some(*action),
                    "{}",
                    binding.keys
                );
            }
        }
    }

    #[test]
    fn unbuilt_rows_are_never_hinted() {
        let hinted: Vec<&str> = hints(Context::Main).iter().map(|(h, _)| h.keys).collect();
        assert_eq!(
            hinted,
            [
                "h/l", ".", "v", "n", "space", "w", "tab", "y", "c", "m", "E", "esc", "?", "q"
            ]
        );
    }

    #[test]
    fn the_keys_copy_formats_may_not_use_are_the_global_keys_the_dialog_keeps() {
        let global: Vec<char> = BINDINGS
            .iter()
            .filter(|b| b.context == Context::Global)
            .flat_map(|b| b.chords.iter())
            .filter_map(|(key, _)| match key {
                Key::Char(c) => Some(*c),
                _ => None,
            })
            .collect();
        assert_eq!(global, teamzones_core::copy::DIALOG_KEYS);
    }

    /// The clauses of the keymap's `Matrix overlay keys:` sentence, each with the keys it names
    /// in backticks (`arrows` standing for the four arrow keys).
    fn matrix_clauses() -> Vec<(String, Vec<Key>)> {
        let (_, after) = KEYS_SPEC
            .split_once("Matrix overlay keys: ")
            .expect("keys.md lists the matrix's keys");
        let (sentence, _) = after
            .split_once(". Mouse in the matrix")
            .expect("the sentence ends before the mouse");
        sentence
            .split(", ")
            .map(|clause| {
                let named = clause.split('`').skip(1).step_by(2).flat_map(|keys| {
                    keys.split_whitespace().map(|word| match word {
                        "space" => Key::Char(' '),
                        "enter" => Key::Enter,
                        "esc" => Key::Esc,
                        other => Key::Char(other.chars().next().unwrap_or(' ')),
                    })
                });
                let arrows = clause
                    .contains(" arrows ")
                    .then_some([Key::Left, Key::Right, Key::Up, Key::Down])
                    .into_iter()
                    .flatten();
                (clause.to_owned(), named.chain(arrows).collect())
            })
            .collect()
    }

    /// Whether an action is the one a clause names.
    type Is = fn(Action) -> bool;

    /// What each clause of the matrix sentence says its keys do.
    fn matrix_action(clause: &str) -> Is {
        let verbs: [(&str, Is); 10] = [
            ("move the column", |a| matches!(a, Action::MoveGroup(_))),
            ("move", |a| matches!(a, Action::Cell(..))),
            ("toggles", |a| a == Action::ToggleCell),
            ("uses the group", |a| a == Action::UseGroup),
            ("copies", |a| a == Action::CopyGroup),
            ("adds a group", |a| a == Action::AddGroup),
            ("renames", |a| a == Action::RenameGroup),
            ("closes the group", |a| a == Action::CloseColumn),
            ("undoes", |a| a == Action::Undo),
            ("finishes", |a| a == Action::FinishMatrix),
        ];
        verbs
            .into_iter()
            .find(|(verb, _)| clause.contains(verb))
            .map_or(|_| false, |(_, is)| is)
    }

    #[test]
    fn the_matrix_bindings_are_the_keys_the_spec_s_matrix_sentence_names() {
        let clauses = matrix_clauses();
        for (clause, keys) in &clauses {
            assert!(!keys.is_empty(), "{clause}");
            for key in keys {
                let action = action_for(Context::Matrix, *key);
                assert!(
                    action.is_some_and(matrix_action(clause)),
                    "{clause}: {key:?} does {action:?}"
                );
            }
        }
        let named: Vec<Key> = clauses.into_iter().flat_map(|(_, keys)| keys).collect();
        let bound = BINDINGS
            .iter()
            .filter(|b| b.context == Context::Matrix)
            .flat_map(|b| b.chords.iter())
            .filter(|(_, action)| *action != Action::Help);
        for (key, action) in bound {
            assert!(
                named.contains(key),
                "{key:?} ({action:?}) is not in the spec"
            );
        }
    }

    /// The clauses of the keymap's `Map mode keys:` sentence, each with the keys it names in
    /// backticks; a clause naming no key (a list of directions) belongs to the one before it.
    fn map_clauses() -> Vec<(String, Vec<Key>)> {
        let (_, after) = KEYS_SPEC
            .split_once("Map mode keys: ")
            .expect("keys.md lists map mode's keys");
        let (sentence, _) = after
            .split_once(". The footer")
            .expect("the sentence ends before the footer");
        let clauses = sentence
            .split(", ")
            .fold(Vec::<String>::new(), |mut acc, part| {
                match acc.last_mut() {
                    Some(last) if !part.contains('`') => *last = format!("{last}, {part}"),
                    _ => acc.push(part.to_owned()),
                }
                acc
            });
        clauses
            .into_iter()
            .map(|clause| {
                let named = clause.split('`').skip(1).step_by(2).flat_map(|keys| {
                    keys.split_whitespace().map(|word| match word {
                        "esc" => Key::Esc,
                        other => Key::Char(other.chars().next().unwrap_or(' ')),
                    })
                });
                let arrows = clause
                    .contains(" arrows ")
                    .then_some([Key::Left, Key::Down, Key::Up, Key::Right])
                    .into_iter()
                    .flatten();
                let keys = named.chain(arrows).collect();
                (clause, keys)
            })
            .collect()
    }

    /// The pan each direction word names, east and south positive.
    fn pan_towards(word: &str) -> Option<Action> {
        match word {
            "west" => Some(Action::Pan(-1, 0)),
            "south" => Some(Action::Pan(0, 1)),
            "north" => Some(Action::Pan(0, -1)),
            "east" => Some(Action::Pan(1, 0)),
            _ => None,
        }
    }

    /// What each map clause says its keys do: the pan clause's keys in the order its directions
    /// are listed (the four letters, then the four arrows).
    fn map_actions(clause: &str, keys: &[Key]) -> Vec<Action> {
        let one: [(&str, Action); 5] = [
            ("zooms in", Action::Zoom(1)),
            ("zooms out", Action::Zoom(-1)),
            ("resets", Action::ResetMap),
            ("opens help", Action::Help),
            ("leaves map mode", Action::Unwind),
        ];
        if clause.contains(" pan ") {
            let directions: Vec<Action> = clause
                .split(|c: char| !c.is_alphabetic())
                .filter_map(pan_towards)
                .collect();
            return directions
                .iter()
                .cycle()
                .take(keys.len())
                .copied()
                .collect();
        }
        one.into_iter()
            .find(|(verb, _)| clause.contains(verb))
            .map(|(_, action)| vec![action; keys.len()])
            .unwrap_or_default()
    }

    #[test]
    fn the_map_mode_bindings_are_the_keys_the_spec_s_map_sentence_names() {
        let clauses = map_clauses();
        for (clause, keys) in &clauses {
            let actions = map_actions(clause, keys);
            assert!(!keys.is_empty() && actions.len() == keys.len(), "{clause}");
            for (key, expected) in keys.iter().zip(actions) {
                assert_eq!(
                    action_for(Context::Map, *key),
                    Some(expected),
                    "{clause}: {key:?}"
                );
            }
        }
        let named: Vec<Key> = clauses.into_iter().flat_map(|(_, keys)| keys).collect();
        let bound = BINDINGS
            .iter()
            .filter(|b| b.context == Context::Map)
            .flat_map(|b| b.chords.iter());
        for (key, action) in bound {
            assert!(
                named.contains(key),
                "{key:?} ({action:?}) is not in the spec"
            );
        }
        for key in [Key::Char('q'), Key::Ctrl('c')] {
            assert_eq!(action_for(Context::Map, key), Some(Action::Quit), "{key:?}");
        }
    }

    #[test]
    fn ctrl_c_quits_even_in_a_prompt() {
        assert_eq!(
            action_for(Context::Prompt, Key::Ctrl('c')),
            Some(Action::Quit)
        );
    }
}
