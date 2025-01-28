(start_left_primary) @ui.cursor.primary
(end_right_primary) @ui.cursor.primary

(start_left) @ui.cursor
(end_right) @ui.cursor

[
  (end_left_primary)
  (start_right_primary)  
  (end_left)
  (start_right)  
] @ui.selection

(left_primary (text) @ui.selection.primary)
(right_primary (text) @ui.selection.primary)
(left (text) @ui.selection)
(right (text) @ui.selection)

(ERROR) @error
