Insert mode bindings are somewhat minimal by default. Helix is designed to
be a modal editor, and this is reflected in the user experience and internal
mechanics. For example, changes to the text are only saved for undos when
escaping from insert mode to normal mode. For this reason, new users are
strongly encouraged to learn the modal editing paradigm to get the smoothest
experience.






However, if you really want navigation in insert mode, this is supported. An
example config that gives the ability to use arrow keys while still in insert
mode:

```toml
[keys.insert]
"up" = "move_line_up"
"down" = "move_line_down"
"left" = "move_char_left"
"right" = "move_char_right"
"C-b" = "move_char_left"
"C-f" = "move_char_right"
"A-b" = "move_prev_word_end"
"C-left" = "move_prev_word_end"
"A-f" = "move_next_word_start"
"C-right" = "move_next_word_start"
"A-<" = "goto_file_start"
"A->" = "goto_file_end"
"pageup" = "page_up"
"pagedown" = "page_down"
"home" = "goto_line_start"
"C-a" = "goto_line_start"
"end" = "goto_line_end_newline"
"C-e" = "goto_line_end_newline"
"A-left" = "goto_line_start"
```
