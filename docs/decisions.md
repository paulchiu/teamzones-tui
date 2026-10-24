# Decisions

What was chosen and why, for someone changing the code. Append only: a reversed decision gets a new entry that supersedes it, and the old heading gains a `Superseded by D<n>` line, keeping its reasoning. Record invariants, not values. Detail that moves (glyphs, widths, copy) lives in [docs/spec/](spec/), where tests parse it, and an entry links that section instead of restating it.

Each entry has three lines. `Held by` names the test, lint or `just` recipe that holds the decision, or says `nothing yet`.

## D1: A region is a load, not a clock (2026-10-24)

Decision: a place adds a trading state to the say line, never a zone, and `--zones` (and `c z`) gives the old `ttz` shape. The `say apps 3pm` row of the [CLI golden table](spec/cli.md#cli-golden-table) shows it.

Why: Paul picked concept V3-B, which makes this the deciding call.

Held by: nothing yet

## D2: The where block never collapses (2026-10-24)

Decision: The where block's height is fixed by the whole roster, and small terminals fold rows instead.

Why: Switching groups never moves anything on screen.

Held by: nothing yet

## D3: Home counts in every group that asks who (2026-10-24)

Decision: Home is counted whenever the marks ask who. In a places-only group, home is the axis only.

Why: A deploy should not be proposed while Paul is off.

Held by: nothing yet

## D4: Load comes from hand-written trading profiles only (2026-10-24)

Decision: A place's load is read from its `trading` profile and nothing else.

Why: No data source exists yet.

Held by: nothing yet

## D5: One zone per region (2026-10-24)

Decision: Each place has exactly one zone. Split a region by adding places (`US-E`, `US-W`). See [config](spec/config.md#data-model-and-config).

Why: Cheap in this model, and nothing needs a union yet.

Held by: nothing yet

## D6: Off-peak means not at peak (2026-10-24)

Decision: `where = "off-peak"` (the default) counts a place that is not at peak. `where = "quiet"` counts a place only while it is closed.

Why: With the sample profiles, AU, UK and US are never all closed at once.

Held by: nothing yet

## D7: Names and paths (2026-10-24)

Decision: Repo `paulchiu/teamzones-tui`, crates `teamzones` and `teamzones-core`, binary `teamzones`, Homebrew formula `teamzones`, MIT licence, public. Config and state paths are in [config](spec/config.md#files-state-and-lifecycle).

Why: "teamzones TUI" as agreed. The old project's binary is `tz`, so nothing collides.

Held by: `crates/teamzones/tests/version.rs` holds the binary name, and `config_path_prints_the_resolved_path` in `crates/teamzones/tests/cli.rs` holds the `TEAMZONES_CONFIG` override; nothing yet holds the default config and state paths

## D8: Mouse capture is on by default (2026-10-24)

Decision: The TUI captures the mouse at start, and `M` releases it for text selection.

Why: The slider is a mouse surface. This is the deliberate opposite of repon ADR 0024 ([GitHub](https://github.com/paulchiu/repon/blob/a4b26cc543af925c8529157ca24b0855c01210d5/docs/adr/0024-repon-releases-what-it-enables-and-holds-mouse-capture-off.md)).

Held by: nothing yet

## D9: Clipboard falls back from arboard to OSC 52 to the notice row (2026-10-24)

Decision: Copying tries `arboard` first, then OSC 52. If both fail, the notice row shows the text. See the notice row in [screen](spec/screen.md).

Why: mossmd already uses arboard, and OSC 52 reaches the local machine over SSH.

Held by: `clipboard::tests::the_fallback_gets_the_text_only_when_the_primary_fails` in `teamzones` holds the arboard then OSC 52 order; nothing yet holds the notice row (the TUI milestone)

## D10: say prints and copies (2026-10-24)

Decision: `teamzones say` prints the line to stdout and copies it unless `--no-copy` is given or stdout is not a TTY. When it copies, `📋 copied` goes to stderr.

Why: Keeps the `ttz` habit and keeps stdout clean for scripts.

Held by: `cli::tests` in `teamzones` (`say_on_a_tty_copies_the_line_and_says_so_on_stderr`, `say_with_no_copy_on_a_tty_copies_nothing`, `say_into_a_pipe_copies_nothing`)

## D11: Travel is set in config, never inferred (2026-10-24)

Decision: Travel is `[travel] here` plus `until`, written by `teamzones here`. It is never inferred from the system time zone.

Why: Paul's Mac reports `Australia/Brisbane` all year.

Held by: `here_writes_travel_after_the_untouched_file_and_clear_removes_it` in `crates/teamzones/tests/cli.rs` holds the write by `teamzones here`; nothing yet holds that the system zone is never consulted while travelling

## D12: Time is computed on a 15-minute grid (2026-10-24)

Decision: Range edges, `n` targets and range states are all evaluated on 15-minute slots aligned to UTC quarter hours.

Why: The screen and the jump agree exactly at 120 columns.

Held by: the `clock::tests` slot tests in `teamzones-core` (quarter-hour alignment, the first slot after now, half-open ranges)

## D13: The core never renders (2026-10-24)

Decision: `teamzones-core` computes everything a screen or CLI line needs and depends on no terminal, rendering or clipboard crate.

Why: A rendering-agnostic core can be tested with plain values, as in repon ADR 0005 ([GitHub](https://github.com/paulchiu/repon/blob/a4b26cc543af925c8529157ca24b0855c01210d5/docs/adr/0005-rendering-agnostic-core.md)).

Held by: `just check-core-isolation`

## D14: One binding table feeds the footer, help and dispatch (2026-10-24)

Decision: Every key's footer hint, help line and dispatch come from one `BINDINGS` table.

Why: Hints that are separate strings drift from what the keys do.

Held by: `tui::keys::tests::compiled_table_matches_the_keymap_spec_row_for_row` and `a_row_dispatches_only_when_its_milestone_names_m3` in `teamzones`, plus the binding meta-tests `every_hinted_key_dispatches_in_its_context`, `no_advertised_key_acts_inside_a_prompt_and_each_is_typed_instead` and `each_modal_context_swallows_the_keys_it_does_not_advertise`

## D15: update is pure and effects run in one layer (2026-10-24)

Decision: `update(state, msg)` returns the new state and a list of effects. Clipboard, file writes, the editor and timers run only in the effects layer.

Why: A pure update is testable without a terminal, a clipboard or a file system.

Held by: `tui::tests::the_update_layer_names_no_io_crate` in `teamzones`, which scans `update.rs` and the `app.rs`, `prompt.rs`, `notice.rs` and `say.rs` it calls into

## D16: No config file, or one holding only comments, means the defaults (2026-10-24)

Decision: When there is no config file, or the file holds only comments and blank lines, home is the system zone (named after it, with no hours) and there are no people, places or groups. A file with any table in it must have `[home]`, and a file that exists but cannot be read or parsed is an error naming its path, never the defaults.

Why: The commented template `config init` writes has to behave exactly like no file, and an unreadable file answering as "nothing there" is the defect blubat fixed twice.

Held by: `config::tests` in `teamzones-core` (no file, a file of only comments, a table without home) and `write::tests::the_template_as_written_is_the_same_config_as_no_file` in `teamzones`

## D17: Place levels are closed, open and busy, and a trading profile may rename them (2026-10-25)

Decision: The default level words are `closed`, `open` and `busy`, replacing `quiet`, `off-peak` and `peak`, and `where` takes `open` and `closed`. A trading profile's `labels` renames the three words wherever its places appear, and the where legend shows them only when every place shares them. The old keywords still load as aliases (`off-peak` as `open`, `quiet` as `closed`), and D6's meaning is unchanged: `open` as a question counts a place that is not busy, closed included. See [config](spec/config.md#data-model-and-config) and [copy](spec/copy.md).

Why: The old words are hospitality words, and an office or support desk reads better as "office hours" or "trading hours". Paul chose the generic defaults at the hands-on round after M3.

Held by: `config::tests::where_takes_the_new_keywords_and_the_old_ones_as_aliases` and the `labels` tests in `config::tests` in `teamzones-core`, `copy::tests::each_place_speaks_its_own_profile_s_labels`, the copy and CLI golden tables, and `tui::tests::the_where_legend_shows_labels_every_place_shares_and_the_defaults_otherwise` in `teamzones`

## D18: City names show in full whenever the label has room (2026-10-25)

Decision: The who block's city column is sized to the longest city and cuts with a mark only when a row would otherwise lose its time. The column still appears only when the label is at least 32 columns, so at 120 columns (a 24-column label) there is no city column at all. See the row label in [screen](spec/screen.md).

Why: A fixed 9-column cap cut `Johannesburg` to `Johannes.` on wide terminals with plenty of spare room.

Held by: `tui::tests::the_city_column_shows_the_whole_city_when_the_label_has_room` and `the_city_column_cuts_with_a_mark_only_when_the_times_need_the_room` in `teamzones`

## D19: The copy box holds only what y copies (2026-10-25)

Decision: The box at the bottom is titled `times for <group>` and contains only the say line. The focused row's detail sits on its own dim line directly above it, and the summary sits in the box's bottom border, so nothing inside the box is left out of the copy. See the say panel in [screen](spec/screen.md).

Why: With the detail inside a border that said `y copies this line`, the box read as if all of it was copied.

Held by: `tui::tests::the_copy_box_holds_only_the_line_y_copies_and_the_detail_sits_above_it` in `teamzones`

## D20: The footer follows the mode (2026-10-25)

Decision: The footer shows the keys that matter for the current mode: a `range` badge and only the range keys while drawing a range, `esc clear range` with a committed range, and `. now` away from now. Mode hints drop last, after everything but `? help` and `q quit`. `1-9 group` leaves the footer and stays in help. See the footer in [screen](spec/screen.md).

Why: A footer that never changed never said what mode you were in or how to get out of it.

Held by: the `footer::tests` mode tests and `tui::tests::range_mode_leads_the_footer_with_its_badge_and_keys_and_keeps_the_main_keys` in `teamzones`, plus the binding meta-test `every_hinted_key_dispatches_in_its_context`

## D21: Copy-as shows exactly what each key copies (2026-10-25)

Decision: The `c` menu is a dialog that previews every format's line for the current time or range, with a short description under each. The preview and the copy come from one function, so what is shown is what is copied, and only the preview is ever cut to fit. See the copy-as dialog in [screen](spec/screen.md).

Why: A menu of format names made you try each one to see what it copied.

Held by: `tui::tests::each_copy_as_key_copies_its_preview_and_closes_the_dialog` and `a_preview_too_wide_for_the_dialog_is_cut_with_a_mark_but_copies_whole` in `teamzones`
