`find_next_char`

Waits for another keypress, then moves and
selects forward, stopping at the first
instance of the pressed key. Can take
a count, which will cause it to stop
at the nth instance of the keypress,
rather than the first.

--- Examples ---

The cursor moves forward, stopping at 'c'
and selecting everything along the way.
┌───────────────────────┐  c  ┌───────────────────────┐
│ This i[s] a sentence. │ --> │ This i(s a sentenc]e. │
└───────────────────────┘     └───────────────────────┘

The cursor is not stopped by line breaks.
┌───────────────────────────┐     ┌────────────────────────────┐
│ This is the fi[r]st line. │  Q  │ This is the fi(rst line.   │
│ This second line has a Q. │ --> │ This second line has a Q]. │
└───────────────────────────┘     └────────────────────────────┘
