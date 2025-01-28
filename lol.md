`copy_selection_on_prev_line`

Copies the current primary selection to the first previous line long enough to accomodate it.

# Examples

The selection is copied from line 2 to line 1.

Before:

```helix
This is text #[|on line 1]#.
This is text on line 2.
```

Command: `C`

After:

```helix
This is text #(|on line 1)#.
This is text #[|on line 2]#.
```
