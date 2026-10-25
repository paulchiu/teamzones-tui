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

Superseded by D17, which renames the keywords to `open` and `closed` and keeps this meaning.

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

Held by: `tui::keys::tests::compiled_table_matches_the_keymap_spec_row_for_row` and `a_row_dispatches_only_when_its_milestone_is_built` in `teamzones`, plus the binding meta-tests `every_hinted_key_dispatches_in_its_context`, `no_advertised_key_acts_inside_a_prompt_and_each_is_typed_instead` and `each_modal_context_swallows_the_keys_it_does_not_advertise`

## D15: update is pure and effects run in one layer (2026-10-24)

Decision: `update(state, msg)` returns the new state and a list of effects. Clipboard, file writes, the editor and timers run only in the effects layer.

Why: A pure update is testable without a terminal, a clipboard or a file system.

Held by: `tui::tests::the_update_layer_names_no_io_crate` in `teamzones`, which scans `update.rs` and the `app.rs`, `prompt.rs`, `notice.rs` and `say.rs` it calls into

## D16: No config file, or one holding only comments, means the defaults (2026-10-24)

Decision: When there is no config file, or the file holds only comments and blank lines, home is the system zone (named after it, with no hours) and there are no people, places or groups. A file with any table in it must have `[home]`, and a file that exists but cannot be read or parsed is an error naming its path, never the defaults.

Why: The commented template `config init` writes has to behave exactly like no file, and an unreadable file answering as "nothing there" is the defect blubat fixed twice.

Held by: `config::tests` in `teamzones-core` (no file, a file of only comments, a table without home) and `write::tests::the_template_as_written_is_the_same_config_as_no_file` in `teamzones`

## D17: Place levels are closed, open and busy, and a trading profile may rename them (2026-10-25)

Decision: The default level words are `closed`, `open` and `busy`, replacing `quiet`, `off-peak` and `peak`, and `where` takes `open` and `closed`. A trading profile's `labels` renames the three words wherever its places appear, and the where legend shows them only when every place shares them. The old keywords still load as aliases (`off-peak` as `open`, `quiet` as `closed`), and D6's meaning is unchanged: `open` as a question counts a place that is not busy, closed included. The say clause names each place by its real state, so a closed place that counts reads `closed in UK`, never `open in UK`. See [config](spec/config.md#data-model-and-config) and [copy](spec/copy.md).

Why: The old words are hospitality words, and an office or support desk reads better as "office hours" or "trading hours". Paul chose the generic defaults at the hands-on round after M3. `off-peak` was true of a closed place, but `open` and a profile's label name a level, so the clause must use the place's own level to stay true.

Held by: `config::tests::where_takes_the_new_keywords_and_the_old_ones_as_aliases` and the `labels` tests in `config::tests` in `teamzones-core`, `copy::tests::each_place_speaks_its_own_profile_s_labels` and `a_counting_place_is_named_by_its_real_state_not_the_question`, the copy and CLI golden tables, and `tui::tests::the_where_legend_shows_labels_every_place_shares_and_the_defaults_otherwise` in `teamzones`

## D18: City names show in full whenever the label has room (2026-10-25)

Decision: The who block's city column is sized to the longest city and cuts with a mark only when a row would otherwise lose its time. When the column appears at all is unchanged, and the widths are in the row label and columns rules of [screen](spec/screen.md).

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

Refined by D24, which leaves out a row whose line repeats an earlier one.

Decision: The `c` menu is a dialog that previews every format's line for the current time or range, with a short description under each. The preview and the copy come from one function, so what is shown is what is copied, and only the preview is ever cut to fit. See the copy-as dialog in [screen](spec/screen.md).

Why: A menu of format names made you try each one to see what it copied.

Held by: `tui::tests::each_copy_as_key_copies_its_preview_and_closes_the_dialog` and `a_preview_too_wide_for_the_dialog_is_cut_with_a_mark_but_copies_whole` in `teamzones`

## D22: The range's columns may carry the selection background, provisionally (2026-10-25)

Superseded by D26, which makes the range style a user setting and keeps `columns` as the default.

Decision: The selection background covers the focused row and the range's columns (every lane, the ruler and the answer rows), as the mockups always drew it. This is provisional: the shading is hard to read, so with `TEAMZONES_DEBUG=1`, `F2` cycles five range-marking styles (columns, edges, lines, underline, dim) and the footer names the current one. Under `NO_COLOR` the styles that need colour (columns, edges and dim) fall back to lines. The choice is local and never saved. Once Paul picks a style, a follow-up removes the others and the debug key. See [theme](spec/theme.md).

Why: The theme text said the selected row was the only background, while the screen mockups shaded the range's columns too. Paul kept the shading at the hands-on round after M3, pending a side-by-side comparison.

Held by: `tui::tests::each_range_style_marks_the_range_its_own_way`, `under_no_color_the_styles_that_need_colour_fall_back_to_edge_lines` and `f2_cycles_the_range_styles_only_in_debug_and_the_footer_names_the_style` in `teamzones`

## D23: Copy-as formats are data, and the default one is the times box, y and say (2026-10-25)

Decision: Every copy-as format is a value (key, label, who, date, names, places), the built-ins `y d w t z s a` included, and `[[copy]]` tables add, replace or hide them. Exactly one format is the default, and its line is the `times for <group>` box, what `y` copies and what `teamzones say` prints, with say's flags adding to it. A `focused-first` format cannot be the default, because `say` has no focused row, and with `names` on it joins the rest with ` / ` rather than nesting parentheses. The keys `?` and `q` are kept for the dialog, a key must be one you can type, and a hidden format takes no field but `key`. Labels are plain words, and the dialog adds the focused row's name to a `focused-first` label. See [copy](spec/copy.md#formats) and [config](spec/config.md#data-model-and-config).

Why: Paul asked to define and override formats at the second hands-on round. The issue's proposed names were kept (`who = "focused-first"`, `places = "states" | "zones" | "none"`). A focused-first default was rejected rather than given a CLI meaning, since any meaning for it in `say` would be a guess. An override of `y` that leaves `default` unset keeps `y` the default, so relabelling it is not an error, while `default = false` on it, or hiding it, with no other default is rejected.

Held by: the `[[copy]]` tests in `config::tests`, `copy::tests::the_built_in_formats_are_the_spec_formats_table_row_for_row` and `a_focused_first_format_with_names_never_nests_parentheses` in `teamzones-core`, `write::tests::the_uncommented_copy_tables_are_exactly_the_built_in_formats`, `tui::keys::tests::the_keys_copy_formats_may_not_use_are_the_global_keys_the_dialog_keeps`, `tui::tests::the_default_format_is_the_times_box_line_and_what_y_copies` and the copy-table dialog tests in `teamzones`, and `say_prints_the_default_copy_format_and_its_flags_add_to_it` in `crates/teamzones/tests/cli.rs`

## D24: No copy line names the same time twice, and the dialog never repeats a line (2026-10-25)

Decision: Zone entries that render identically are merged in every format, so `t` with a row in the axis zone copies its time once. The copy-as dialog leaves out a format whose line equals an earlier row's, and that format's key still copies the line.

Why: On a group with no places, `y`, `z` and `s` showed three identical rows, and `t` on home copied `10:30-11:15am AEDT (10:30-11:15am AEDT)`.

Held by: `copy::tests::the_focus_line_for_a_row_in_the_axis_zone_names_the_time_once` in `teamzones-core`, and `tui::tests::a_hidden_duplicate_row_s_key_still_copies_the_same_line` and `t_with_home_focused_copies_the_time_once` in `teamzones`

## D25: llms.txt names every config field (2026-10-25)

Decision: `llms.txt` at the repo root teaches a coding agent to write a config, gathering colleagues' cities, zones and hours from real sources, and it names every field the config parser accepts.

Why: An agent writing a config for its user needs one page, and a page that lags the parser teaches fields that no longer exist or misses new ones.

Held by: `config::tests::llms_txt_names_every_field_the_config_accepts` in `teamzones-core`, which reads the field lists from the parser's own tables

## D26: The range style is a user setting, and columns stays the default (2026-10-25)

Decision: `[theme] range` picks how the range is marked (`columns`, `edges`, `lines`, `underline` or `focus`), defaulting to `columns`. `R` cycles it for the session without writing config, and the footer names the new style briefly. `TEAMZONES_DEBUG` and `F2` are gone. The `NO_COLOR` fallbacks from D22 still hold. The style D22 called `dim` is `focus`, the name Paul used. See [theme](spec/theme.md).

Why: `F2` behind a debug variable was awkward to reach, and Paul wanted the styles as a real choice rather than a pending pick.

Held by: `config::tests::theme_range_picks_the_range_style_and_defaults_to_columns` in `teamzones-core`, and `tui::tests::capital_r_cycles_the_range_style_for_the_session_and_the_footer_names_it_briefly`, `each_range_style_marks_the_range_its_own_way` and `under_no_color_the_styles_that_need_colour_fall_back_to_edge_lines` in `teamzones`

## D27: The copy-as dialog's keys come from the config's formats (2026-10-25)

Decision: Inside the copy-as dialog, a key dispatches when it names one of the loaded formats, ahead of the `BINDINGS` table, which has no copy-menu row. The dialog itself lists each key beside its preview, so the footer shows only `? help` and `q quit` there, and help opened from it lists only the global keys. When two rows per format do not fit, the dialog shows one row per format and drops the descriptions. This narrows D14 for this one context. See the copy-as dialog in [screen](spec/screen.md).

Why: With formats from config, a fixed chord list would advertise a hidden key and miss a new one.

Held by: `tui::tests::each_modal_context_swallows_the_keys_it_does_not_advertise` (which takes the dialog's own keys from the config), `a_copy_table_replaces_a_built_in_and_hide_removes_one_from_the_dialog_and_its_key` and `a_dialog_too_tall_for_two_rows_a_format_lists_every_key_on_one_row_each`, and `tui::help::tests::help_from_the_copy_dialog_lists_only_the_global_keys` in `teamzones`

## D28: Hands-on feedback rounds count as built milestones in the keymap (2026-10-25)

Decision: A keymap row whose milestone names a feedback round (`FB2`) is built, just as one naming `M3` is, so `R` dispatches and is advertised before M4.

Why: Feedback rounds add keys between the numbered milestones, and the keymap test must accept them without marking the unbuilt M4 to M6 rows as built.

Held by: `tui::keys::tests::a_row_dispatches_only_when_its_milestone_is_built` in `teamzones`
