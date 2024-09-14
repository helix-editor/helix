| Name | Description | Default keybinds |
| --- | --- | --- |
| `no_op` | Do nothing |  |
| `move_char_left` | Move left | normal: `` h ``, `` <left> ``, insert: `` <left> `` |
| `move_char_right` | Move right | normal: `` <right> ``, `` l ``, insert: `` <right> `` |
| `move_line_up` | Move up | normal: `` gk `` |
| `move_line_down` | Move down | normal: `` gj `` |
| `move_visual_line_up` | Move up | normal: `` <up> ``, `` k ``, insert: `` <up> `` |
| `move_visual_line_down` | Move down | normal: `` <down> ``, `` j ``, insert: `` <down> `` |
| `extend_char_left` | Extend left | select: `` h ``, `` <left> `` |
| `extend_char_right` | Extend right | select: `` <right> ``, `` l `` |
| `extend_line_up` | Extend up | select: `` gk `` |
| `extend_line_down` | Extend down | select: `` gj `` |
| `extend_visual_line_up` | Extend up | select: `` <up> ``, `` k `` |
| `extend_visual_line_down` | Extend down | select: `` <down> ``, `` j `` |
| `copy_selection_on_next_line` | Copy selection on next line | normal: `` C ``, normal: `` C `` |
| `copy_selection_on_prev_line` | Copy selection on previous line | normal: `` <A-C> ``, normal: `` <A-C> `` |
| `move_next_word_start` | Move to start of next word | normal: `` w `` |
| `move_prev_word_start` | Move to start of previous word | normal: `` b `` |
| `move_next_word_end` | Move to end of next word | normal: `` e `` |
| `move_prev_word_end` | Move to end of previous word |  |
| `move_next_long_word_start` | Move to start of next long word | normal: `` W `` |
| `move_prev_long_word_start` | Move to start of previous long word | normal: `` B `` |
| `move_next_long_word_end` | Move to end of next long word | normal: `` E `` |
| `move_prev_long_word_end` | Move to end of previous long word |  |
| `move_next_sub_word_start` | Move to start of next sub word |  |
| `move_prev_sub_word_start` | Move to start of previous sub word |  |
| `move_next_sub_word_end` | Move to end of next sub word |  |
| `move_prev_sub_word_end` | Move to end of previous sub word |  |
| `move_parent_node_end` | Move to end of the parent node | normal: `` <A-e> `` |
| `move_parent_node_start` | Move to beginning of the parent node | normal: `` <A-b> `` |
| `extend_next_word_start` | Extend to start of next word | select: `` w `` |
| `extend_prev_word_start` | Extend to start of previous word | select: `` b `` |
| `extend_next_word_end` | Extend to end of next word | select: `` e `` |
| `extend_prev_word_end` | Extend to end of previous word |  |
| `extend_next_long_word_start` | Extend to start of next long word | select: `` W `` |
| `extend_prev_long_word_start` | Extend to start of previous long word | select: `` B `` |
| `extend_next_long_word_end` | Extend to end of next long word | select: `` E `` |
| `extend_prev_long_word_end` | Extend to end of prev long word |  |
| `extend_next_sub_word_start` | Extend to start of next sub word |  |
| `extend_prev_sub_word_start` | Extend to start of previous sub word |  |
| `extend_next_sub_word_end` | Extend to end of next sub word |  |
| `extend_prev_sub_word_end` | Extend to end of prev sub word |  |
| `extend_parent_node_end` | Extend to end of the parent node | select: `` <A-e> `` |
| `extend_parent_node_start` | Extend to beginning of the parent node | select: `` <A-b> `` |
| `find_till_char` | Move till next occurrence of char | normal: `` t `` |
| `find_next_char` | Move to next occurrence of char | normal: `` f `` |
| `extend_till_char` | Extend till next occurrence of char | select: `` t `` |
| `extend_next_char` | Extend to next occurrence of char | select: `` f `` |
| `till_prev_char` | Move till previous occurrence of char | normal: `` T `` |
| `find_prev_char` | Move to previous occurrence of char | normal: `` F `` |
| `extend_till_prev_char` | Extend till previous occurrence of char | select: `` T `` |
| `extend_prev_char` | Extend to previous occurrence of char | select: `` F `` |
| `repeat_last_motion` | Repeat last motion | normal: `` <A-.> ``, normal: `` <A-.> `` |
| `replace` | Replace with new char | normal: `` r ``, normal: `` r `` |
| `switch_case` | Switch (toggle) case | normal: `` ~ ``, normal: `` ~ `` |
| `switch_to_uppercase` | Switch to uppercase | normal: `` <A-`> ``, normal: `` <A-`> `` |
| `switch_to_lowercase` | Switch to lowercase | normal: `` ` ``, normal: `` ` `` |
| `page_up` | Move page up | normal: `` z<C-b> ``, `` z<pageup> ``, `` <C-b> ``, `` <pageup> ``, `` Z<pageup> ``, `` Z<C-b> ``, normal: `` z<C-b> ``, `` z<pageup> ``, `` <C-b> ``, `` <pageup> ``, `` Z<pageup> ``, `` Z<C-b> ``, insert: `` <pageup> `` |
| `page_down` | Move page down | normal: `` z<C-f> ``, `` z<pagedown> ``, `` <pagedown> ``, `` Z<C-f> ``, `` Z<pagedown> ``, `` <C-f> ``, normal: `` z<C-f> ``, `` z<pagedown> ``, `` <pagedown> ``, `` Z<C-f> ``, `` Z<pagedown> ``, `` <C-f> ``, insert: `` <pagedown> `` |
| `half_page_up` | Move half page up |  |
| `half_page_down` | Move half page down |  |
| `page_cursor_up` | Move page and cursor up |  |
| `page_cursor_down` | Move page and cursor down |  |
| `page_cursor_half_up` | Move page and cursor half up | normal: `` z<C-u> ``, `` z<backspace> ``, `` <C-u> ``, `` Z<C-u> ``, `` Z<backspace> ``, normal: `` z<C-u> ``, `` z<backspace> ``, `` <C-u> ``, `` Z<C-u> ``, `` Z<backspace> `` |
| `page_cursor_half_down` | Move page and cursor half down | normal: `` z<space> ``, `` z<C-d> ``, `` <C-d> ``, `` Z<C-d> ``, `` Z<space> ``, normal: `` z<space> ``, `` z<C-d> ``, `` <C-d> ``, `` Z<C-d> ``, `` Z<space> `` |
| `select_all` | Select whole document | normal: `` % ``, normal: `` % `` |
| `select_regex` | Select all regex matches inside selections | normal: `` s ``, normal: `` s `` |
| `split_selection` | Split selections on regex matches | normal: `` S ``, normal: `` S `` |
| `split_selection_on_newline` | Split selection on newlines | normal: `` <A-s> ``, normal: `` <A-s> `` |
| `merge_selections` | Merge selections | normal: `` <A-minus> ``, normal: `` <A-minus> `` |
| `merge_consecutive_selections` | Merge consecutive selections | normal: `` <A-_> ``, normal: `` <A-_> `` |
| `search` | Search for regex pattern | normal: `` / ``, `` z/ ``, `` Z/ ``, normal: `` / ``, `` z/ ``, `` Z/ `` |
| `rsearch` | Reverse search for regex pattern | normal: `` z? ``, `` ? ``, `` Z? ``, normal: `` z? ``, `` ? ``, `` Z? `` |
| `search_next` | Select next search match | normal: `` n ``, `` zn ``, `` Zn ``, select: `` zn ``, `` Zn `` |
| `search_prev` | Select previous search match | normal: `` zN ``, `` N ``, `` ZN ``, select: `` zN ``, `` ZN `` |
| `extend_search_next` | Add next search match to selection | select: `` n `` |
| `extend_search_prev` | Add previous search match to selection | select: `` N `` |
| `search_selection` | Use current selection as search pattern | normal: `` * ``, normal: `` * `` |
| `make_search_word_bounded` | Modify current search to make it word bounded |  |
| `global_search` | Global search in workspace folder | normal: `` <space>/ ``, normal: `` <space>/ `` |
| `extend_line` | Select current line, if already selected, extend to another line based on the anchor |  |
| `extend_line_below` | Select current line, if already selected, extend to next line | normal: `` x ``, normal: `` x `` |
| `extend_line_above` | Select current line, if already selected, extend to previous line |  |
| `select_line_above` | Select current line, if already selected, extend or shrink line above based on the anchor |  |
| `select_line_below` | Select current line, if already selected, extend or shrink line below based on the anchor |  |
| `extend_to_line_bounds` | Extend selection to line bounds | normal: `` X ``, normal: `` X `` |
| `shrink_to_line_bounds` | Shrink selection to line bounds | normal: `` <A-x> ``, normal: `` <A-x> `` |
| `delete_selection` | Delete selection | normal: `` d ``, normal: `` d `` |
| `delete_selection_noyank` | Delete selection without yanking | normal: `` <A-d> ``, normal: `` <A-d> `` |
| `change_selection` | Change selection | normal: `` c ``, normal: `` c `` |
| `change_selection_noyank` | Change selection without yanking | normal: `` <A-c> ``, normal: `` <A-c> `` |
| `collapse_selection` | Collapse selection into single cursor | normal: `` ; ``, normal: `` ; `` |
| `flip_selections` | Flip selection cursor and anchor | normal: `` <A-;> ``, normal: `` <A-;> `` |
| `ensure_selections_forward` | Ensure all selections face forward | normal: `` <A-:> ``, normal: `` <A-:> `` |
| `insert_mode` | Insert before selection | normal: `` i ``, normal: `` i `` |
| `append_mode` | Append after selection | normal: `` a ``, normal: `` a `` |
| `command_mode` | Enter command mode | normal: `` : ``, normal: `` : `` |
| `file_picker` | Open file picker | normal: `` <space>f ``, normal: `` <space>f `` |
| `file_picker_in_current_buffer_directory` | Open file picker at current buffer's directory |  |
| `file_picker_in_current_directory` | Open file picker at current working directory | normal: `` <space>F ``, normal: `` <space>F `` |
| `code_action` | Perform code action | normal: `` <space>a ``, normal: `` <space>a `` |
| `buffer_picker` | Open buffer picker | normal: `` <space>b ``, normal: `` <space>b `` |
| `jumplist_picker` | Open jumplist picker | normal: `` <space>j ``, normal: `` <space>j `` |
| `symbol_picker` | Open symbol picker | normal: `` <space>s ``, normal: `` <space>s `` |
| `changed_file_picker` | Open changed file picker | normal: `` <space>g ``, normal: `` <space>g `` |
| `select_references_to_symbol_under_cursor` | Select symbol references | normal: `` <space>h ``, normal: `` <space>h `` |
| `workspace_symbol_picker` | Open workspace symbol picker | normal: `` <space>S ``, normal: `` <space>S `` |
| `diagnostics_picker` | Open diagnostic picker | normal: `` <space>d ``, normal: `` <space>d `` |
| `workspace_diagnostics_picker` | Open workspace diagnostic picker | normal: `` <space>D ``, normal: `` <space>D `` |
| `last_picker` | Open last picker | normal: `` <space>' ``, normal: `` <space>' `` |
| `insert_at_line_start` | Insert at start of line | normal: `` I ``, normal: `` I `` |
| `insert_at_line_end` | Insert at end of line | normal: `` A ``, normal: `` A `` |
| `open_below` | Open new line below selection | normal: `` o ``, normal: `` o `` |
| `open_above` | Open new line above selection | normal: `` O ``, normal: `` O `` |
| `normal_mode` | Enter normal mode | normal: `` <esc> ``, select: `` v ``, normal: `` <esc> `` |
| `select_mode` | Enter selection extend mode | normal: `` v `` |
| `exit_select_mode` | Exit selection mode | select: `` <esc> `` |
| `goto_definition` | Goto definition | normal: `` gd ``, normal: `` gd `` |
| `goto_declaration` | Goto declaration | normal: `` gD ``, normal: `` gD `` |
| `add_newline_above` | Add newline above | normal: `` [<space> ``, normal: `` [<space> `` |
| `add_newline_below` | Add newline below | normal: `` ]<space> ``, normal: `` ]<space> `` |
| `goto_type_definition` | Goto type definition | normal: `` gy ``, normal: `` gy `` |
| `goto_implementation` | Goto implementation | normal: `` gi ``, normal: `` gi `` |
| `goto_file_start` | Goto line number <n> else file start | normal: `` gg ``, normal: `` gg `` |
| `goto_file_end` | Goto file end |  |
| `goto_file` | Goto files/URLs in selections | normal: `` gf ``, normal: `` gf `` |
| `goto_file_hsplit` | Goto files in selections (hsplit) | normal: `` <space>wf ``, `` <C-w>f ``, normal: `` <space>wf ``, `` <C-w>f `` |
| `goto_file_vsplit` | Goto files in selections (vsplit) | normal: `` <space>wF ``, `` <C-w>F ``, normal: `` <space>wF ``, `` <C-w>F `` |
| `goto_reference` | Goto references | normal: `` gr ``, normal: `` gr `` |
| `goto_window_top` | Goto window top | normal: `` gt ``, normal: `` gt `` |
| `goto_window_center` | Goto window center | normal: `` gc ``, normal: `` gc `` |
| `goto_window_bottom` | Goto window bottom | normal: `` gb ``, normal: `` gb `` |
| `goto_last_accessed_file` | Goto last accessed file | normal: `` ga ``, normal: `` ga `` |
| `goto_last_modified_file` | Goto last modified file | normal: `` gm ``, normal: `` gm `` |
| `goto_last_modification` | Goto last modification | normal: `` g. ``, normal: `` g. `` |
| `goto_line` | Goto line | normal: `` G ``, normal: `` G `` |
| `goto_last_line` | Goto last line | normal: `` ge ``, normal: `` ge `` |
| `goto_first_diag` | Goto first diagnostic | normal: `` [D ``, normal: `` [D `` |
| `goto_last_diag` | Goto last diagnostic | normal: `` ]D ``, normal: `` ]D `` |
| `goto_next_diag` | Goto next diagnostic | normal: `` ]d ``, normal: `` ]d `` |
| `goto_prev_diag` | Goto previous diagnostic | normal: `` [d ``, normal: `` [d `` |
| `goto_next_change` | Goto next change | normal: `` ]g ``, normal: `` ]g `` |
| `goto_prev_change` | Goto previous change | normal: `` [g ``, normal: `` [g `` |
| `goto_first_change` | Goto first change | normal: `` [G ``, normal: `` [G `` |
| `goto_last_change` | Goto last change | normal: `` ]G ``, normal: `` ]G `` |
| `goto_line_start` | Goto line start | normal: `` <home> ``, `` gh ``, select: `` gh ``, insert: `` <home> `` |
| `goto_line_end` | Goto line end | normal: `` <end> ``, `` gl ``, select: `` gl `` |
| `goto_next_buffer` | Goto next buffer | normal: `` gn ``, normal: `` gn `` |
| `goto_previous_buffer` | Goto previous buffer | normal: `` gp ``, normal: `` gp `` |
| `goto_line_end_newline` | Goto newline at line end | insert: `` <end> `` |
| `goto_first_nonwhitespace` | Goto first non-blank in line | normal: `` gs ``, normal: `` gs `` |
| `trim_selections` | Trim whitespace from selections | normal: `` _ ``, normal: `` _ `` |
| `extend_to_line_start` | Extend to line start | select: `` <home> `` |
| `extend_to_first_nonwhitespace` | Extend to first non-blank in line |  |
| `extend_to_line_end` | Extend to line end | select: `` <end> `` |
| `extend_to_line_end_newline` | Extend to line end |  |
| `signature_help` | Show signature help |  |
| `smart_tab` | Insert tab if all cursors have all whitespace to their left; otherwise, run a separate command. | insert: `` <tab> `` |
| `insert_tab` | Insert tab char | insert: `` <S-tab> `` |
| `insert_newline` | Insert newline char | insert: `` <ret> ``, `` <C-j> `` |
| `delete_char_backward` | Delete previous char | insert: `` <backspace> ``, `` <C-h> ``, `` <S-backspace> `` |
| `delete_char_forward` | Delete next char | insert: `` <C-d> ``, `` <del> `` |
| `delete_word_backward` | Delete previous word | insert: `` <C-w> ``, `` <A-backspace> `` |
| `delete_word_forward` | Delete next word | insert: `` <A-d> ``, `` <A-del> `` |
| `kill_to_line_start` | Delete till start of line | insert: `` <C-u> `` |
| `kill_to_line_end` | Delete till end of line | insert: `` <C-k> `` |
| `undo` | Undo change | normal: `` u ``, normal: `` u `` |
| `redo` | Redo change | normal: `` U ``, normal: `` U `` |
| `earlier` | Move backward in history | normal: `` <A-u> ``, normal: `` <A-u> `` |
| `later` | Move forward in history | normal: `` <A-U> ``, normal: `` <A-U> `` |
| `commit_undo_checkpoint` | Commit changes to new checkpoint | insert: `` <C-s> `` |
| `yank` | Yank selection | normal: `` y ``, normal: `` y `` |
| `yank_to_clipboard` | Yank selections to clipboard | normal: `` <space>y ``, normal: `` <space>y `` |
| `yank_to_primary_clipboard` | Yank selections to primary clipboard |  |
| `yank_joined` | Join and yank selections |  |
| `yank_joined_to_clipboard` | Join and yank selections to clipboard |  |
| `yank_main_selection_to_clipboard` | Yank main selection to clipboard | normal: `` <space>Y ``, normal: `` <space>Y `` |
| `yank_joined_to_primary_clipboard` | Join and yank selections to primary clipboard |  |
| `yank_main_selection_to_primary_clipboard` | Yank main selection to primary clipboard |  |
| `replace_with_yanked` | Replace with yanked text | normal: `` R ``, normal: `` R `` |
| `replace_selections_with_clipboard` | Replace selections by clipboard content | normal: `` <space>R ``, normal: `` <space>R `` |
| `replace_selections_with_primary_clipboard` | Replace selections by primary clipboard |  |
| `paste_after` | Paste after selection | normal: `` p ``, normal: `` p `` |
| `paste_before` | Paste before selection | normal: `` P ``, normal: `` P `` |
| `paste_clipboard_after` | Paste clipboard after selections | normal: `` <space>p ``, normal: `` <space>p `` |
| `paste_clipboard_before` | Paste clipboard before selections | normal: `` <space>P ``, normal: `` <space>P `` |
| `paste_primary_clipboard_after` | Paste primary clipboard after selections |  |
| `paste_primary_clipboard_before` | Paste primary clipboard before selections |  |
| `indent` | Indent selection | normal: `` <gt> ``, normal: `` <gt> `` |
| `unindent` | Unindent selection | normal: `` <lt> ``, normal: `` <lt> `` |
| `format_selections` | Format selection | normal: `` = ``, normal: `` = `` |
| `join_selections` | Join lines inside selection | normal: `` J ``, normal: `` J `` |
| `join_selections_space` | Join lines inside selection and select spaces | normal: `` <A-J> ``, normal: `` <A-J> `` |
| `keep_selections` | Keep selections matching regex | normal: `` K ``, normal: `` K `` |
| `remove_selections` | Remove selections matching regex | normal: `` <A-K> ``, normal: `` <A-K> `` |
| `align_selections` | Align selections in column | normal: `` & ``, normal: `` & `` |
| `keep_primary_selection` | Keep primary selection | normal: `` , ``, normal: `` , `` |
| `remove_primary_selection` | Remove primary selection | normal: `` <A-,> ``, normal: `` <A-,> `` |
| `completion` | Invoke completion popup | insert: `` <C-x> `` |
| `hover` | Show docs for item under cursor | normal: `` <space>k ``, normal: `` <space>k `` |
| `toggle_comments` | Comment/uncomment selections | normal: `` <space>c ``, `` <C-c> ``, normal: `` <space>c ``, `` <C-c> `` |
| `toggle_line_comments` | Line comment/uncomment selections | normal: `` <space><A-c> ``, normal: `` <space><A-c> `` |
| `toggle_block_comments` | Block comment/uncomment selections | normal: `` <space>C ``, normal: `` <space>C `` |
| `rotate_selections_forward` | Rotate selections forward | normal: `` ) ``, normal: `` ) `` |
| `rotate_selections_backward` | Rotate selections backward | normal: `` ( ``, normal: `` ( `` |
| `rotate_selection_contents_forward` | Rotate selection contents forward | normal: `` <A-)> ``, normal: `` <A-)> `` |
| `rotate_selection_contents_backward` | Rotate selections contents backward | normal: `` <A-(> ``, normal: `` <A-(> `` |
| `reverse_selection_contents` | Reverse selections contents |  |
| `expand_selection` | Expand selection to parent syntax node | normal: `` <A-o> ``, `` <A-up> ``, normal: `` <A-o> ``, `` <A-up> `` |
| `shrink_selection` | Shrink selection to previously expanded syntax node | normal: `` <A-down> ``, `` <A-i> ``, normal: `` <A-down> ``, `` <A-i> `` |
| `select_next_sibling` | Select next sibling in the syntax tree | normal: `` <A-n> ``, `` <A-right> ``, normal: `` <A-n> ``, `` <A-right> `` |
| `select_prev_sibling` | Select previous sibling the in syntax tree | normal: `` <A-left> ``, `` <A-p> ``, normal: `` <A-left> ``, `` <A-p> `` |
| `select_all_siblings` | Select all siblings of the current node | normal: `` <A-a> ``, normal: `` <A-a> `` |
| `select_all_children` | Select all children of the current node | normal: `` <A-I> ``, `` <S-A-down> ``, normal: `` <A-I> ``, `` <S-A-down> `` |
| `jump_forward` | Jump forward on jumplist | normal: `` <C-i> ``, `` <tab> ``, normal: `` <C-i> ``, `` <tab> `` |
| `jump_backward` | Jump backward on jumplist | normal: `` <C-o> ``, normal: `` <C-o> `` |
| `save_selection` | Save current selection to jumplist | normal: `` <C-s> ``, normal: `` <C-s> `` |
| `jump_view_right` | Jump to right split | normal: `` <space>wl ``, `` <space>w<right> ``, `` <space>w<C-l> ``, `` <C-w>l ``, `` <C-w><C-l> ``, `` <C-w><right> ``, normal: `` <space>wl ``, `` <space>w<right> ``, `` <space>w<C-l> ``, `` <C-w>l ``, `` <C-w><C-l> ``, `` <C-w><right> `` |
| `jump_view_left` | Jump to left split | normal: `` <space>w<left> ``, `` <space>w<C-h> ``, `` <space>wh ``, `` <C-w>h ``, `` <C-w><C-h> ``, `` <C-w><left> ``, normal: `` <space>w<left> ``, `` <space>w<C-h> ``, `` <space>wh ``, `` <C-w>h ``, `` <C-w><C-h> ``, `` <C-w><left> `` |
| `jump_view_up` | Jump to split above | normal: `` <space>w<C-k> ``, `` <space>wk ``, `` <space>w<up> ``, `` <C-w>k ``, `` <C-w><up> ``, `` <C-w><C-k> ``, normal: `` <space>w<C-k> ``, `` <space>wk ``, `` <space>w<up> ``, `` <C-w>k ``, `` <C-w><up> ``, `` <C-w><C-k> `` |
| `jump_view_down` | Jump to split below | normal: `` <space>wj ``, `` <space>w<C-j> ``, `` <space>w<down> ``, `` <C-w>j ``, `` <C-w><C-j> ``, `` <C-w><down> ``, normal: `` <space>wj ``, `` <space>w<C-j> ``, `` <space>w<down> ``, `` <C-w>j ``, `` <C-w><C-j> ``, `` <C-w><down> `` |
| `swap_view_right` | Swap with right split | normal: `` <space>wL ``, `` <C-w>L ``, normal: `` <space>wL ``, `` <C-w>L `` |
| `swap_view_left` | Swap with left split | normal: `` <space>wH ``, `` <C-w>H ``, normal: `` <space>wH ``, `` <C-w>H `` |
| `swap_view_up` | Swap with split above | normal: `` <space>wK ``, `` <C-w>K ``, normal: `` <space>wK ``, `` <C-w>K `` |
| `swap_view_down` | Swap with split below | normal: `` <space>wJ ``, `` <C-w>J ``, normal: `` <space>wJ ``, `` <C-w>J `` |
| `transpose_view` | Transpose splits | normal: `` <space>wt ``, `` <space>w<C-t> ``, `` <C-w><C-t> ``, `` <C-w>t ``, normal: `` <space>wt ``, `` <space>w<C-t> ``, `` <C-w><C-t> ``, `` <C-w>t `` |
| `rotate_view` | Goto next window | normal: `` <space>ww ``, `` <space>w<C-w> ``, `` <C-w>w ``, `` <C-w><C-w> ``, normal: `` <space>ww ``, `` <space>w<C-w> ``, `` <C-w>w ``, `` <C-w><C-w> `` |
| `rotate_view_reverse` | Goto previous window |  |
| `hsplit` | Horizontal bottom split | normal: `` <space>ws ``, `` <space>w<C-s> ``, `` <C-w><C-s> ``, `` <C-w>s ``, normal: `` <space>ws ``, `` <space>w<C-s> ``, `` <C-w><C-s> ``, `` <C-w>s `` |
| `hsplit_new` | Horizontal bottom split scratch buffer | normal: `` <space>wn<C-s> ``, `` <space>wns ``, `` <C-w>n<C-s> ``, `` <C-w>ns ``, normal: `` <space>wn<C-s> ``, `` <space>wns ``, `` <C-w>n<C-s> ``, `` <C-w>ns `` |
| `vsplit` | Vertical right split | normal: `` <space>w<C-v> ``, `` <space>wv ``, `` <C-w>v ``, `` <C-w><C-v> ``, normal: `` <space>w<C-v> ``, `` <space>wv ``, `` <C-w>v ``, `` <C-w><C-v> `` |
| `vsplit_new` | Vertical right split scratch buffer | normal: `` <space>wn<C-v> ``, `` <space>wnv ``, `` <C-w>nv ``, `` <C-w>n<C-v> ``, normal: `` <space>wn<C-v> ``, `` <space>wnv ``, `` <C-w>nv ``, `` <C-w>n<C-v> `` |
| `wclose` | Close window | normal: `` <space>wq ``, `` <space>w<C-q> ``, `` <C-w><C-q> ``, `` <C-w>q ``, normal: `` <space>wq ``, `` <space>w<C-q> ``, `` <C-w><C-q> ``, `` <C-w>q `` |
| `wonly` | Close windows except current | normal: `` <space>w<C-o> ``, `` <space>wo ``, `` <C-w>o ``, `` <C-w><C-o> ``, normal: `` <space>w<C-o> ``, `` <space>wo ``, `` <C-w>o ``, `` <C-w><C-o> `` |
| `select_register` | Select register | normal: `` " ``, normal: `` " `` |
| `insert_register` | Insert register | insert: `` <C-r> `` |
| `align_view_middle` | Align view middle | normal: `` zm ``, `` Zm ``, normal: `` zm ``, `` Zm `` |
| `align_view_top` | Align view top | normal: `` zt ``, `` Zt ``, normal: `` zt ``, `` Zt `` |
| `align_view_center` | Align view center | normal: `` zc ``, `` zz ``, `` Zz ``, `` Zc ``, normal: `` zc ``, `` zz ``, `` Zz ``, `` Zc `` |
| `align_view_bottom` | Align view bottom | normal: `` zb ``, `` Zb ``, normal: `` zb ``, `` Zb `` |
| `scroll_up` | Scroll view up | normal: `` zk ``, `` z<up> ``, `` Z<up> ``, `` Zk ``, normal: `` zk ``, `` z<up> ``, `` Z<up> ``, `` Zk `` |
| `scroll_down` | Scroll view down | normal: `` z<down> ``, `` zj ``, `` Z<down> ``, `` Zj ``, normal: `` z<down> ``, `` zj ``, `` Z<down> ``, `` Zj `` |
| `match_brackets` | Goto matching bracket | normal: `` mm ``, normal: `` mm `` |
| `surround_add` | Surround add | normal: `` ms ``, normal: `` ms `` |
| `surround_replace` | Surround replace | normal: `` mr ``, normal: `` mr `` |
| `surround_delete` | Surround delete | normal: `` md ``, normal: `` md `` |
| `select_textobject_around` | Select around object | normal: `` ma ``, normal: `` ma `` |
| `select_textobject_inner` | Select inside object | normal: `` mi ``, normal: `` mi `` |
| `goto_next_function` | Goto next function | normal: `` ]f ``, normal: `` ]f `` |
| `goto_prev_function` | Goto previous function | normal: `` [f ``, normal: `` [f `` |
| `goto_next_class` | Goto next type definition | normal: `` ]t ``, normal: `` ]t `` |
| `goto_prev_class` | Goto previous type definition | normal: `` [t ``, normal: `` [t `` |
| `goto_next_parameter` | Goto next parameter | normal: `` ]a ``, normal: `` ]a `` |
| `goto_prev_parameter` | Goto previous parameter | normal: `` [a ``, normal: `` [a `` |
| `goto_next_comment` | Goto next comment | normal: `` ]c ``, normal: `` ]c `` |
| `goto_prev_comment` | Goto previous comment | normal: `` [c ``, normal: `` [c `` |
| `goto_next_test` | Goto next test | normal: `` ]T ``, normal: `` ]T `` |
| `goto_prev_test` | Goto previous test | normal: `` [T ``, normal: `` [T `` |
| `goto_next_entry` | Goto next pairing | normal: `` ]e ``, normal: `` ]e `` |
| `goto_prev_entry` | Goto previous pairing | normal: `` [e ``, normal: `` [e `` |
| `goto_next_paragraph` | Goto next paragraph | normal: `` ]p ``, normal: `` ]p `` |
| `goto_prev_paragraph` | Goto previous paragraph | normal: `` [p ``, normal: `` [p `` |
| `dap_launch` | Launch debug target | normal: `` <space>Gl ``, normal: `` <space>Gl `` |
| `dap_restart` | Restart debugging session | normal: `` <space>Gr ``, normal: `` <space>Gr `` |
| `dap_toggle_breakpoint` | Toggle breakpoint | normal: `` <space>Gb ``, normal: `` <space>Gb `` |
| `dap_continue` | Continue program execution | normal: `` <space>Gc ``, normal: `` <space>Gc `` |
| `dap_pause` | Pause program execution | normal: `` <space>Gh ``, normal: `` <space>Gh `` |
| `dap_step_in` | Step in | normal: `` <space>Gi ``, normal: `` <space>Gi `` |
| `dap_step_out` | Step out | normal: `` <space>Go ``, normal: `` <space>Go `` |
| `dap_next` | Step to next | normal: `` <space>Gn ``, normal: `` <space>Gn `` |
| `dap_variables` | List variables | normal: `` <space>Gv ``, normal: `` <space>Gv `` |
| `dap_terminate` | End debug session | normal: `` <space>Gt ``, normal: `` <space>Gt `` |
| `dap_edit_condition` | Edit breakpoint condition on current line | normal: `` <space>G<C-c> ``, normal: `` <space>G<C-c> `` |
| `dap_edit_log` | Edit breakpoint log message on current line | normal: `` <space>G<C-l> ``, normal: `` <space>G<C-l> `` |
| `dap_switch_thread` | Switch current thread | normal: `` <space>Gst ``, normal: `` <space>Gst `` |
| `dap_switch_stack_frame` | Switch stack frame | normal: `` <space>Gsf ``, normal: `` <space>Gsf `` |
| `dap_enable_exceptions` | Enable exception breakpoints | normal: `` <space>Ge ``, normal: `` <space>Ge `` |
| `dap_disable_exceptions` | Disable exception breakpoints | normal: `` <space>GE ``, normal: `` <space>GE `` |
| `shell_pipe` | Pipe selections through shell command | normal: `` | ``, normal: `` | `` |
| `shell_pipe_to` | Pipe selections into shell command ignoring output | normal: `` <A-|> ``, normal: `` <A-|> `` |
| `shell_insert_output` | Insert shell command output before selections | normal: `` ! ``, normal: `` ! `` |
| `shell_append_output` | Append shell command output after selections | normal: `` <A-!> ``, normal: `` <A-!> `` |
| `shell_keep_pipe` | Filter selections with shell predicate | normal: `` $ ``, normal: `` $ `` |
| `suspend` | Suspend and return to shell | normal: `` <C-z> ``, normal: `` <C-z> `` |
| `rename_symbol` | Rename symbol | normal: `` <space>r ``, normal: `` <space>r `` |
| `increment` | Increment item under cursor | normal: `` <C-a> ``, normal: `` <C-a> `` |
| `decrement` | Decrement item under cursor | normal: `` <C-x> ``, normal: `` <C-x> `` |
| `record_macro` | Record macro | normal: `` Q ``, normal: `` Q `` |
| `replay_macro` | Replay macro | normal: `` q ``, normal: `` q `` |
| `command_palette` | Open command palette | normal: `` <space>? ``, normal: `` <space>? `` |
| `goto_word` | Jump to a two-character label | normal: `` gw `` |
| `extend_to_word` | Extend to a two-character label | select: `` gw `` |
