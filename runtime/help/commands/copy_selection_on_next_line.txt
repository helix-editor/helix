`copy_selection_on_next_line`

Copies the current primary selection to the next line long enough to accomodate it.

--- Examples ---

The selection is copied from line 1 to line 2.
┌───────────────────────────┐     ┌───────────────────────────┐
│ This is text (on line 1]. │ --> │ This is text (on line 1]. │
│ This is text on line 2.   │     │ This is text (on line 2]. │
└───────────────────────────┘     └───────────────────────────┘

The selection duplication skips line 2 because it is too short.
┌──────────────────────────────────┐     ┌──────────────────────────────────┐
│ This is a longer li(ne of t]ext. │     │ This is a longer li(ne of t]ext. │
│ This is a shorter line.          │ --> │ This is a shorter line.          │
│ This is another longer line.     │     │ This is another lon(ger lin]e.   │
└──────────────────────────────────┘     └──────────────────────────────────┘
