# Decisions

What was chosen and why, for someone changing the code. Append only: a reversed decision gets a new entry that supersedes it, and the old heading gains a `Superseded by D<n>` line, keeping its reasoning. Record invariants, not values. Detail that moves (glyphs, widths, copy) lives in [docs/spec/](spec/), where tests parse it, and an entry links that section instead of restating it.

Each entry has three lines. `Held by` names the test, lint or `just` recipe that holds the decision, or says `nothing yet`.

## D1: A region is a load, not a clock (2026-10-24)

Superseded by D49, which adds each counted place's clock to the line and keeps its state.

Decision: a place adds a trading state to the say line, never a zone, and `--zones` (and `c z`) gives the old `ttz` shape. The `say apps 3pm` row of the [CLI golden table](spec/cli.md#cli-golden-table) shows it.

Why: Paul picked concept V3-B, which makes this the deciding call.

Held by: nothing yet

## D2: The where block never collapses (2026-10-24)

Decision: The where block's height is fixed by the whole roster, and small terminals fold rows instead.

Why: Switching groups never moves anything on screen.

Held by: `tui::tests::switching_groups_never_moves_the_ruler_the_borders_or_the_say_panel`, `tui::tests::map::with_the_map_off_a_group_switch_never_moves_the_ruler_the_borders_or_the_times_box` and, folded at 80x20, `tui::tests::fold::at_80_by_20_switching_groups_never_moves_a_block` in `teamzones`

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

Held by: `tui::tests::mouse::capital_m_releases_the_mouse_and_says_so_until_m_again_recaptures_it` and `quitting_releases_mouse_capture_and_then_leaves_the_alternate_screen` in `crates/teamzones/tests/terminal_restoration.rs` in `teamzones`

## D9: Clipboard falls back from arboard to OSC 52 to the notice row (2026-10-24)

Superseded by D46 on Linux only, where OSC 52 is written as well as arboard; macOS keeps this order.

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

Superseded in part by D45: the detail line sits directly above the box only when there are no spare rows.

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

## D29: The label stays in proportion to the track, and narrow rows stack (2026-10-25)

Superseded in part by D51: below 60 columns the blocks are the clock list at any height, so rows stack only from 60 to 79 columns and the plain list is gone.

Decision: From 80 columns the label is never wider than the track, and from 120 columns it is at most half of it. Column i is the same instant in every row, the ruler and the track. Below 80 columns each row stacks its label over a full-width lane when the height allows; when it does not, the label sits beside the track while it still fits, and the plain list appears only when neither does. See the Columns, Ruler and Folding bullets in [screen](spec/screen.md).

Why: A fixed 96-column track handed every extra column to the label, which outgrew the track on wide terminals, and the side-by-side split left too little room below 80 columns. Paul asked for a label to track ratio of about 1:2 and a stacked layout when narrow. Whole columns per hour keep hours evenly spaced on the ruler. Stacking doubles the grid's rows and folding (M6) is not built yet, so a short narrow terminal keeps its lanes beside the labels, as it did before, rather than losing them to the list.

Held by: `tui::layout::tests::from_80_columns_the_label_never_outgrows_the_track_and_from_120_stays_within_half` and `the_split_matches_every_example_in_the_screen_spec`, and `tui::tests::at_250_columns_every_grid_row_marks_the_range_on_the_same_columns`, `at_250_columns_the_cursor_and_fit_marks_sit_on_their_instants`, `ruler_labels_space_out_with_the_cell_size`, `below_80_columns_each_row_stacks_its_label_over_a_full_width_lane`, `from_60_to_79_columns_without_the_height_to_stack_lanes_sit_beside_the_labels` and `below_60_columns_without_the_height_to_stack_the_screen_is_a_plain_list` in `teamzones`

## D30: m flips the map until the next resize, and the overlay is not a modal (2026-10-25)

Superseded in part by D45: `[map] show = "always"` needs two spare rows, since one always sits over the times box.

Decision: `m` flips the map's automatic choice only until the terminal is resized, when the choice is worked out afresh. The overlay `m` opens without room is drawn over the grid while every main key still works under it, so scrubbing moves the night side; `esc` closes it before unwinding anything else. A configured `[map] show = "always"` takes any spare rows, and `never` makes `m` show the map where the hiding rule finds room. See Map hiding in [screen](spec/screen.md).

Why: Keeping a flip across a resize would turn a map hidden at 120x36 into an overlay at 120x24, which nobody asked for. A modal overlay would add a context whose only key is `esc`, and scrubbing under the map is the point of opening it.

Held by: `tui::layout::tests::m_flips_each_configured_choice_to_the_other_or_to_an_overlay_without_room`, the `tui::tests::map` hide, overlay and resize tests, `tui::tests::map::main_keys_still_work_under_the_map_overlay` and `tui::tests::mouse::the_wheel_scrubs_under_the_map_overlay_and_a_click_inside_it_does_nothing` in `teamzones`

## D31: Every built mouse equivalent sends its key's messages (2026-10-25)

Decision: A pointer event is translated into the messages its keymap row's key sends (a wheel notch is `l`, a click on a strip name is its number), hit-tested against the areas the view draws at the terminal's last size. Drags become repeated edge or move keys, and drawing a range is `v`, moves and `enter`. Only clicking a row label, putting the cursor down for a drawn range and clicking a fitting run have no key, so they get their own messages. A press on the grid that never moves does nothing, except on a fitting cell of the track.

Why: The mouse then cannot drift from the keys: whatever a key does, its mouse equivalent does by sending it. A test reads the keymap's mouse column, so a new mouse row fails until it is covered.

Held by: `tui::tests::mouse::every_built_mouse_equivalent_in_the_keymap_sends_what_its_key_does` and, for the matrix's own list, `every_matrix_mouse_equivalent_in_the_keymap_sends_what_its_key_does` in `teamzones`

## D32: Edits keep every byte they do not touch, and never write over a file changed elsewhere (2026-10-25)

Decision: Every TUI edit (`w`, `+`, `x`, `=`, the matrix, `a`, `e`, `d`) rewrites the config text through `toml_edit` in `write.rs`, touching only the keys whose values change, and is parsed back before it is adopted. The effects layer replaces the file only while it still holds the text the app last read or wrote, so an edit made outside teamzones is never lost; the notice says so and `r` reloads.

Why: The config is the user's own file, with their comments and layout, and the TUI holds its text in memory between reads.

Held by: `tui::tests::edit::space_on_uk_then_w_on_team_writes_its_places_and_nothing_else` and `the_effects_layer_writes_the_bytes_and_refuses_a_file_changed_behind_its_back` in `teamzones`

## D33: Undo is one value holding the whole state before the action, and a close waits out its window unwritten (2026-10-25)

Decision: The single undo holds the config, its text, any pending write, the lens, the marks and the focus as they were, never a diff, and expires 10 s after the action. `x` changes the screen at once but writes only when the window passes or on quit, so `u` before then never touches the file. Any other write inside that window (`w`, `+`, `a`, `e`, `d`, `r`, `E`, or opening the matrix) writes the pending close with it; `u` on that later write puts the file back to the bytes it held before it, and the close waits out its window again. The matrix writes all its changes once, when it closes, with one undo back to how it opened, and `u` inside it steps back one change.

Why: A diff can fail to apply after a later edit, and a whole state cannot. Writing a close only after its window keeps an undone close off the disk entirely.

Held by: `tui::tests::edit::u_within_ten_seconds_writes_the_file_back_to_its_previous_bytes`, `x_on_deploy_leaves_the_strip_at_once_and_writes_only_after_ten_seconds`, `u_after_x_puts_the_group_back_in_its_slot_and_nothing_is_written`, `u_on_a_write_made_while_a_close_waits_puts_the_file_back_and_the_close_still_waits`, `a_close_pending_when_the_matrix_opens_is_written_then_and_never_with_its_changes` and `matrix_changes_are_written_together_on_esc_and_the_old_name_still_says` in `teamzones`

## D34: The editor handoff parks the key reader and drains its input before reading again (2026-10-25)

Decision: `E` runs the editor on the event loop's own thread, so no key is read while it runs. On return the terminal is taken back whatever the editor did, and everything buffered, plus anything arriving within 20 ms, is discarded before the loop reads again. Events, terminal modes and the editor opener are injected, so the handoff is tested without a terminal.

Why: blubat's reader raced the editor for stdin, and late terminal replies read as keys after it exited.

Held by: `tui::tests::edit::capital_e_hands_the_terminal_to_the_editor_and_drops_its_input_then_reloads` and `a_failing_editor_still_gives_the_terminal_back_and_reloads_nothing` in `teamzones`, and the pty test `the_editor_gets_a_released_terminal_and_the_tui_takes_it_back_after` in `crates/teamzones/tests/terminal_restoration.rs`

## D35: `a` asks person or place first, and the picker writes only the fields that changed (2026-10-25)

Decision: `a` opens a two-key menu, `p` person and `l` place, before the picker, since focus alone cannot add the first place. The picker's fields are text prompts stepped through with `enter`. An edit writes only the fields whose text changed, renaming the row in every group when its name changes, and a city typed for a person who had `label`, `tz` and `at` replaces those keys. A new place without a marker takes the first bundled city in its zone. The drift tag's menu has keys too, `w` save and `r` revert, so it acts on its own keys like every modal.

Why: The spec names the picker's fields but not how to choose its kind or move between fields, and text prompts swallow every key but `esc` and `enter`.

Held by: `tui::tests::edit::a_p_adds_a_person_after_the_last_one_through_the_picker`, `a_l_adds_a_place_after_the_last_one_through_the_picker`, `e_edits_the_focused_person_s_hours_and_writes_only_that_line` and `tui::tests::each_modal_context_swallows_the_keys_it_does_not_advertise` in `teamzones`

## D36: A drift in `where` is shown but stays out of the who-expression (2026-10-25)

Decision: A `where` changed with `=` shows as `=closed` (or `=context`, `=open`) in the strip's drift tag and the `w` notice, but the who-expression leaves it out, so `+` offers a name from the members alone and the matrix's CLI line names the group as saved.

Why: `teamzones say` has no syntax for a `where` override, so an expression carrying one could not be pasted into the CLI, and the drift tag is where an unsaved change has to be visible.

Held by: `tui::tests::edit::equals_cycles_where_as_drift_until_w_skipping_context_without_people` in `teamzones`

## D37: A reload drops the undo (2026-10-25)

Decision: A successful `r`, or the reload after `E`, clears the pending undo and any pending close, adopting the file as it now stands.

Why: What the undo would put back predates the file now on disk, so `u` after a reload would write over edits made in the editor or elsewhere.

Held by: `tui::tests::edit::a_reload_drops_the_undo_so_u_never_writes_over_the_file_read` in `teamzones`

## D38: The footer fills gaps, so a short lower-priority hint can outlive a longer higher one (2026-10-25)

Decision: The footer keeps its existing fit: it adds hints from the highest priority down and skips any hint that does not fit in the room left, so a shorter lower-priority hint can still fit after a longer higher-priority one is skipped. At 80x24 on regions this keeps `w save` and `tab groups` while the longer `n next fit` is dropped, which differs from the 80x24 caption in [screen](spec/screen.md) ("drops `v`, `n`, `w` and `c`"). This needs Paul's call: either the caption changes or the fit becomes strict.

Why: The fit predates this milestone and the new `w` and `tab` hints only exposed it. Changing the contract or the ladder is Paul's decision, not a feature PR's.

Held by: the `regions_80x24` snapshot in `teamzones`

## D39: The map keeps its proportions and frames its markers once, never per group (2026-10-25)

Decision: A Braille dot spans the same degrees of longitude and latitude, so the map never stretches. The default view fills the width and crops latitude to a band centred on home and the counted markers, held inside 80N to 58S, or shows the whole map centred when the area is tall enough. The view is a stored camera, worked out at start, on `0` in map mode and on resize, and a group switch never recomputes it. See the Map bullets in [screen](spec/screen.md).

Why: On wide terminals the stretched map flattened the continents, and Paul asked for the macOS Clock app's zoom-to-fill. The issue also asked for the band to be centred on the counted markers and for a group switch never to move the map (D2); a band recomputed every frame would satisfy the first and break the second, so the band is framed on the markers counted when the view is set and then held. This supersedes M4's check that the UK, US, AU and Melbourne labels all show at 120x36: that band now crops the UK and Melbourne, so the four-label check runs at 120x48, where the whole map fits.

Held by: `tui::map::tests::a_degree_of_longitude_and_of_latitude_take_the_same_distance_at_every_size`, `the_band_never_shows_beyond_the_map_s_latitudes` and `an_area_taller_than_the_map_centres_it_with_blank_rows_above_and_below`, and `tui::tests::map::the_band_is_centred_on_home_and_the_counted_markers_within_the_map_s_bounds`, `a_group_switch_never_moves_the_map`, `the_map_labels_home_and_each_counted_place_with_its_state_over_the_range` (at 120x48) and the `regions_map_120x37` snapshot in `teamzones`

## D40: Map mode is a modal with its own keys, entered with `z` (2026-10-25)

Decision: `z` opens map mode only while the map is drawn, in place or as the overlay. Map mode answers its own pan, zoom, reset and help keys plus the global quit keys and swallows the rest, with a `map` footer badge like range mode's. `esc` leaves it before closing the overlay, the view outlives it, and a resize that hides the map ends it. In map mode the wheel over the map sends the pan keys and the wheel elsewhere sends nothing, since its time keys (`h`, `l`) pan here.

Why: Panning needs `h j k l`, which the main screen already uses for time and focus, so the keys cannot fall through to main the way range mode's do. `z` (zoom) was free on the main screen.

Held by: `tui::tests::map::z_opens_map_mode_while_the_map_is_drawn_and_its_footer_names_its_keys_until_esc`, `map_mode_works_on_the_overlay_and_esc_leaves_it_before_closing_the_overlay`, `a_resize_that_hides_the_map_leaves_map_mode`, `the_first_k_from_a_band_held_at_the_southern_edge_pans_a_full_step`, `tui::map::tests::a_pan_or_zoom_starts_from_the_band_as_drawn_not_a_centre_past_the_edge`, `tui::keys::tests::the_map_mode_bindings_are_the_keys_the_spec_s_map_sentence_names`, `tui::tests::each_modal_context_swallows_the_keys_it_does_not_advertise` and `tui::tests::mouse::every_built_mouse_equivalent_in_the_keymap_sends_what_its_key_does` in `teamzones`

## D41: The night side's edge is a Braille line in the dawn colour (2026-10-25)

Decision: The map draws the terminator at Braille dot resolution: every dark dot with a lit dot beside, above or below it. The line's cells take the `dawn` theme colour and show only its dots, over the cell shading, so it reads by shape under `mono`. It follows the view and the range's or cursor's instant, and the ascii glyph set, which never draws the map, has no line. See the Map bullets in [screen](spec/screen.md).

Why: A cell background cannot go finer than a cell, so the shading alone drew a staircase, and Paul asked for the Clock app's smooth line. `dawn` already means twilight on the lanes, so no new theme key is needed, and showing only the line's dots in its cells keeps it one dot thick where it crosses a coast.

Held by: `tui::map::tests::the_terminator_s_dots_sit_within_one_dot_of_the_computed_edge_of_night` and `tui::tests::map::the_terminator_draws_in_the_dawn_colour_and_keeps_its_shape_under_mono` in `teamzones`

## D42: With the map off, the blocks move up and the spare rows sit above the notice row (2026-10-25)

Superseded by D45, which keeps the blocks under the header and moves the spare rows to sit over the times box.

Decision: Whenever the map is not drawn in place (hidden by `m`, by `[map] show = "never"`, by the hiding rule, or under `glyphs = "ascii"`), the ruler starts on the row under the header and the rows the map would have taken sit together, blank, between the times box and the notice row. The notice row stays directly above the footer, and the footer stays on the last row. `m` bringing the map back pushes the blocks down again, and the overlay is drawn over the blocks without moving them. Spare rows exist only when every block fits whole, so no block is ever cut short while they show and there is no block to hand them to first. See the layout and Map hiding bullets in [screen](spec/screen.md).

Why: Paul found the header sitting alone over a dozen blank rows when `m` hid the map at about 225x50, and expected the rest of the screen to fill the space. Keeping the notice row next to the footer keeps it where the 80x24 frames put it. Group switches still never move a block (D2), since the layout depends only on the size, the roster and the map's place.

Held by: `tui::layout::tests::a_hidden_map_moves_the_blocks_up_and_leaves_its_rows_above_the_notice`, `tui::tests::map::with_the_map_off_the_blocks_sit_under_the_header_and_the_spare_rows_above_the_notice`, `a_map_shown_never_or_under_ascii_glyphs_leaves_no_gap_under_the_header`, `the_overlay_opens_over_the_blocks_without_moving_them`, `with_the_map_off_a_group_switch_never_moves_the_ruler_the_borders_or_the_times_box` and `tui::tests::mouse::with_the_map_off_a_click_on_a_label_lands_on_the_row_drawn_there` in `teamzones`

## D43: Travel follows the clock, and home's own city is a reference row that is copied but never counted (2026-10-25)

Superseded in part by D49: a places-only line now names its places' zones after the axis.

Decision: Whether `[travel]` holds is decided by now, never by the time on screen, so scrubbing past `until` never moves the axis, while a running screen comes home once the clock passes it. While it holds, the roster has home in the travel city with its own hours, and home's own city as a reference: a dim `⌂` row directly under home, never focusable or counted, drawn with plain day and night. Its zone joins every line whose marks ask who, after the axis in offset order like any other zone, with no name under `c w` (it is where home is not). A places-only line names the axis alone, as before. The map marks home `✈` in the travel city and draws nothing at home's own city. See [config](spec/config.md#data-model-and-config).

Why: The milestone card fixed the shape (`3pm GMT / Tue 2am AEDT / Tue 1am AEST / 5pm SAST`, counted home, an uncounted reference) but not where the reference row sits, whether names cover it or what the time on screen does to travel. An axis that changed while scrubbing would redraw the track under the cursor.

Held by: `config::tests::while_travelling_home_moves_to_the_travel_city_and_its_own_city_is_the_reference`, `from_the_day_after_until_in_the_travel_city_home_is_back_without_a_write` and the `copy::tests::while_travelling_*` tests in `teamzones-core`, the `tui::tests::travel` tests in `teamzones` (`scrubbing_past_until_never_moves_the_axis` for the scrubbing clause), and `while_travelling_say_reads_times_in_the_travel_city_and_still_copies_home_s_zone` in `crates/teamzones/tests/cli.rs`

## D44: A fold row stands for the rows it folds, and its list is an overlay, not a modal (2026-10-25)

Decision: Folding keeps each block's lines in their usual order and turns the last line that fits into `+N more`, with the answer row in its usual place unless counted rows are folded, when it follows the fold row. Focus on a fold row is focus on the first row it folds, so `j` and `k` step onto it and a resize that folds the focused row moves focus to the fold row holding it. Row keys act on no row there: `space`, `e` and `d` refuse with a notice, the detail line describes the fold, and a focused-first copy puts home first. `enter` on it opens a list of the folded rows drawn over the screen while the main keys still work, as the map overlay does (D30), and `enter` again, `esc` (before anything else) or moving focus off it closes it. `enter` no longer commits a range outside range mode, where it did nothing. Before the first resize the size is unknown and nothing folds. See the Folding bullets in [screen](spec/screen.md).

Why: The spec says what folds and in what order, and that `enter` lists the folded rows, but not how focus reaches a row that is not a person or a place, nor how the list closes. Standing for a real row keeps focus a `Row` everywhere, and an overlay keeps `enter`'s keymap row the only binding it needs.

Held by: the `tui::fold::tests` and `tui::layout::tests::at_80_by_20_the_who_block_folds_to_fit_and_everything_else_keeps_its_rows`, `short_of_rows_the_where_block_folds_first_down_to_three_places` and `the_who_block_never_folds_below_home_one_more_row_and_its_answer`, and the `tui::tests::fold` tests (`row_keys_on_the_fold_row_leave_the_roster_and_the_marks_alone` for the row keys) and the `enter` case of `tui::tests::mouse::every_built_mouse_equivalent_in_the_keymap_sends_what_its_key_does` in `teamzones`

## D45: The times box keeps to the bottom, and the spare rows sit over it (2026-10-25)

Decision: The times box, the notice row and the footer keep to the bottom of the screen, with the footer on the last row. The spare rows sit together, blank, between the detail line and the times box. A map drawn in place takes all of them but one, so a blank row separates the detail line from the times box whenever there is any spare row; `[map] show = "always"` therefore needs two. With no spare rows nothing changes: the detail line sits directly over the times box, and folding still decides who gives up rows. The rest of D42 holds: with the map off the ruler starts under the header. This refines D19, whose detail line now sits directly over the box only when there are no spare rows. See the layout and Map hiding bullets in [screen](spec/screen.md).

Why: Paul found the times box jammed against the grid with the free rows under it, between the box and the footer, and asked for room to breathe. Anchoring the box to the bottom puts the space where it reads as a margin. Group switches still never move a block (D2), since the layout depends only on the size, the roster and the map's place.

Held by: `tui::layout::tests::wherever_the_blocks_fit_whole_the_times_box_keeps_to_the_bottom_under_a_blank_row`, `a_hidden_map_moves_the_blocks_up_and_leaves_its_rows_above_the_times_box`, `always_shows_no_map_with_one_spare_row_and_leaves_it_blank_over_the_times_box` and `at_120_by_36_the_map_takes_eleven_of_the_twelve_spare_rows_and_one_sits_over_the_times_box`, and `tui::tests::map::with_the_map_off_the_blocks_sit_under_the_header_and_the_spare_rows_above_the_times_box`, `at_145_by_30_the_spare_rows_collect_over_the_times_box_and_the_footer_takes_the_last_row` and `with_the_map_off_a_group_switch_never_moves_the_ruler_the_borders_or_the_times_box` in `teamzones`

## D46: On Linux a copy goes to arboard and to OSC 52 both (2026-10-25)

Decision: On Linux, `y` and `teamzones say` give the line to arboard and also write it as OSC 52 to `/dev/tty`, and report `copied` when either takes it. Elsewhere the D9 order stands: arboard, then OSC 52 only if arboard fails. The notice row still shows the text when both fail.

Why: On Linux the X11 and Wayland selection belongs to the process that set it, so arboard can report success and the clipboard empties when `teamzones say` exits unless a clipboard manager takes it over. Writing OSC 52 as well lets a terminal that supports it keep the line. Keeping the selection alive with a detached helper (arboard's `SetExtLinux::wait`) needs a forked process this binary cannot make without `unsafe`, and dropping `copied` would leave the common clipboard-manager case reporting nothing. Paul has no Linux machine, so CI's Linux runner is what proves the choice: no test there shows the line surviving on a real desktop.

Held by: `clipboard::tests::on_linux_the_terminal_gets_the_text_even_when_the_system_clipboard_takes_it`, `the_chain_the_binary_copies_through_writes_both_exactly_when_built_for_linux` (which checks the chain `main` builds, so the Linux CI job holds the Linux wiring) and `both_succeeds_when_either_takes_the_text_and_fails_with_both_reasons` in `teamzones`

## D47: The state file keeps the last group, and the TUI writes it only on a clean quit (2026-10-25)

Refined by D50, which keeps each lens's marks there too.

Decision: `state.toml` lives under the XDG state directory (`$XDG_STATE_HOME`, else `~/.local/state`) on every platform, as config does. The TUI reads `last_group` before it takes the terminal and opens on that group, matched case-insensitively, or on `0 all` for `all`; a name the config no longer has opens the first group. It writes `last_group` through `toml_edit` when the user quits, keeping every other key and comment, atomically. A state file that exists but cannot be read, does not parse or holds a non-string `last_group` is an error naming the path, exit 2, and since the TUI never starts nothing is written over it. The map override and mouse capture setting the spec also lists are not kept yet. A failed write on quit is reported on stderr and does not change the exit code.

Why: The spec names the file and says the TUI opens on the last used group but not when it is saved or what a stale name does. Saving on quit keeps the write out of the event loop, and a stale name falling back to the first group matches what a closed group does in the session. Absent is not unreadable, so a broken file is never treated as empty.

Held by: the `state::tests` and `tui::tests::the_screen_opens_on_the_saved_last_group_and_on_the_first_without_one` in `teamzones`, and `the_tui_opens_on_the_group_the_state_file_names_and_saves_the_one_it_quits_on` in `crates/teamzones/tests/terminal_restoration.rs`

## D48: Debug builds panic on request so the pty test can hold the panic hook (2026-10-25)

Decision: With `TEAMZONES_TEST_PANIC` set, a debug build panics right after its first frame. Release builds compile the check out entirely. The pty test sets it and asserts that the output releases mouse capture and leaves the alternate screen before the panic message, and that the pty is back in canonical mode.

Why: The panic hook restores the terminal only on a panic, and nothing in the app panics on purpose. A debug-only hook is the smallest way to make the real binary exercise it without shipping a way to crash a release build.

Held by: `a_panic_releases_mouse_capture_raw_mode_and_the_alternate_screen` in `crates/teamzones/tests/terminal_restoration.rs`

## D49: A counted place adds its clock to the line, and keeps its state after the comma (2026-10-25)

Decision: Every counted place, with a trading profile or without, adds its zone to the default line (the `times for` box, `y` and `teamzones say`), sorted among the people's zones by offset and merged with any that render the same time, and a place with a trading profile still adds its state to the clause. This is `places = "both"`, the built-in `y`, `d`, `w` and `a` formats and the default for a `[[copy]]` table that leaves `places` out. `places = "states"` keeps the old clause-only line for anyone who wants it. `--zones` (`c z`) keeps the places' zones and drops the clause, and `--no-clause` (`c s`) drops the places altogether, leaving the people's times. `--names` (`c w`) names a place inside its zone's parentheses, after the people there, as in `(Paul, AU)` or `(UK)`. The times box summary counts the zones the line names, places' included (`4 zones for 5 people, 1 place`, `3 zones for 3 places`), and a places-only line under `places = "states"`, which names no place clocks, still reads `home zone, 3 places`. See [copy](spec/copy.md).

Why: Paul marked UK, a place with no trading profile, and nothing changed in the line he copies, because D1 let a place contribute only its state. He expects a place he adds to show up in `times for`. Keeping the clause keeps the answer, and keeping `states` as a value keeps D1's line one `[[copy]]` table away. `--no-clause` could have become the same line as `--zones`; dropping the places with it keeps the two flags distinct and keeps `c s` "times only". Naming a place inside its zone's parentheses says which clock it added, without a second list beside the clause.

Held by: the copy and CLI golden tables, `copy::tests::a_counting_place_is_named_by_its_real_state_not_the_question` and `while_travelling_a_places_only_line_names_the_axis_then_the_places_clocks` in `teamzones-core`, `tui::tests::marking_a_place_without_trading_adds_its_clock_to_the_times_box_and_the_summary_counts_it`, `a_places_only_summary_counts_the_zones_its_places_add`, `a_places_only_summary_under_a_states_line_counts_no_zones`, `edit::the_matrix_cursor_says_a_place_without_trading_adds_only_its_clock` and `the_times_box_shows_what_say_prints_for_every_fixed_time_golden_row` in `teamzones`, and `say_with_a_place_group_prints_each_place_s_clock_and_the_flags_keep_or_drop_them` in `crates/teamzones/tests/cli.rs`

## D50: The state file keeps each lens's unsaved marks, by name, and opening the lens puts them back (2026-10-25)

Decision: On a clean quit, `state.toml` gets a `[marks.<lens>]` table for every lens whose marked people or places differ from its config (`0 all` included), holding them by name in config order, and loses the tables of lenses that no longer differ; the write keeps other keys and comments, atomically, with `last_group` (D47). Opening a lens, at start or by switching to it, takes its kept marks back as the working marks, over the lens's own `where`; the existing switch rule then still drops unsaved drift on the way out, with `u`. A lens whose marks match its config keeps nothing, so `w` saving the marks into the group clears them. A kept name or lens the config no longer has is dropped silently, and home stays marked on a lens that asks who even when its kept list lacks home's name; a `[marks]` value of the wrong shape is an error naming the path. A changed `where` (D36) is not kept. See [config](spec/config.md#files-state-and-lifecycle).

Why: Paul marked who and where on `0 all`, which has no group to save into, quit, and lost them. Names rather than ids survive a reload, an edit or a deleted row, which renumber the roster. Keeping the switch's drop and undo means a kept lens behaves exactly like one whose marks were just made, rather than adding per-lens drift that the spec's `dropped team +@UK` notice would no longer describe. `where` has no place in the issue and a who-expression cannot carry it either.

Held by: `state::tests::the_marks_of_each_lens_come_back_as_written_and_other_keys_survive`, `remembering_no_marks_removes_the_marks_table_and_fewer_marks_drop_the_rest` and `marks_of_the_wrong_shape_are_an_error_naming_the_path`, `tui::tests::marks_a_person_and_a_place_on_all_and_keeps_them_for_the_next_run`, `a_switch_to_a_lens_with_kept_marks_puts_them_back_and_u_keeps_them_kept`, `names_and_lenses_the_config_no_longer_has_are_dropped_and_marks_matching_config_keep_nothing`, `kept_marks_without_home_s_name_still_count_home` and `w_saving_the_marks_into_their_group_clears_its_kept_marks` in `teamzones`, and `marks_made_on_all_are_kept_on_quit_and_come_back_on_the_next_run` in `crates/teamzones/tests/terminal_restoration.rs`

## D51: Below 60 columns the blocks are a clock list of cities, and a city row holds focus one person at a time (2026-10-27)

Decision: Below 60 columns, at any height from 10 rows, the who and where blocks are drawn as a clock list: one two-line row per city (people who share a city name and zone share it) and per place, with no lanes, ruler, track or detail line, a one-line slider and the group bar as a title line. It follows the existing mode (the cursor's time, or the range) with no mode of its own. Focus still moves person by person, then through the places, so every row key acts on one person, and a click on a city row focuses its first person. The who block keeps three lines a city, which holds a rule between every two city rows and the `not in` rule whichever cities count, so a group switch never moves the where block; short of room the rules go before anything folds. A person's state word is their lane state at the cursor or the range's first slot, read from the one function the lanes draw from, so `mixed` only ever means the people in a row differ. Cities follow the grid's row order (east to west, or by name under name sort) from the same sort key. The folds keep the existing floors (home's city, a `+N more` line and the answer row in the who block, three places in the where block), so below 20 rows the times box, notice and footer clip at the bottom, as every layout already does at that height, rather than fold further. From 60 columns nothing changes. See the Folding and Clock list bullets in [screen](spec/screen.md).

Why: Paul picked concept A (a phone's world clock) for narrow terminals, where stacked lanes and the plain list cut the group bar off after `0` and gave a track too narrow to read. Grouping by city puts the question "what time is it there" first, and keeping focus on people keeps `space`, `e`, `d` and `enter` meaning what they mean at 60 columns. The issue's mockup drew the title above the slider, rules only between some rows, and no where answer row or detail line; the criteria win where they differ, so the title stays where the who border sits, rules sit between every two city rows or none, and the answer rows keep their place. Dropping the detail line and the where bottom border, as the mockup does, fits the fixture's 44x30 frame without folding.

Held by: `tui::layout::tests::below_60_columns_at_any_height_the_blocks_are_the_clock_list_with_rules_given_room`, `at_44_by_30_the_clock_list_has_one_slider_line_and_no_detail_line` and `short_of_rows_the_clock_list_folds_the_where_block_first_then_the_who_block`, and the `tui::tests::clocks` tests in `teamzones` (`the_44_by_30_team_frame_is_the_screen_spec_s_mockup` against the spec's mockup, `switching_groups_never_moves_the_where_heading_or_the_times_box` for the heights, `cities_run_east_to_west_in_each_section_whatever_their_config_order` and `under_name_sort_cities_run_by_city_name_in_each_section_after_home` for the order, `with_rules_drawn_a_click_lands_on_the_city_row_under_it` for the hit test)
