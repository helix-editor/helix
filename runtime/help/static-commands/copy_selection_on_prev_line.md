`copy_selection_on_prev_line`

Copies the current primary selection to the first previous line long enough to accomodate it.

--- Examples ---

The selection is copied from line 2 to line 1.
┌───────────────────────────┐     ┌───────────────────────────┐
│ This is text on line 1.   │ --> │ This is text (on line 1]. │
│ This is text (on line 2]. │     │ This is text (on line 2]. │
└───────────────────────────┘     └───────────────────────────┘

The selection duplication skips line 2 because it is too short.
┌──────────────────────────────────┐     ┌──────────────────────────────────┐
│ This is a longer line of text.   │     │ This is a longer li(ne of t]ext. │
│ This is a shorter line.          │ --> │ This is a shorter line.          │
│ This is another lon(ger lin]e.   │     │ This is another lon(ger lin]e.   │
└──────────────────────────────────┘     └──────────────────────────────────┘
