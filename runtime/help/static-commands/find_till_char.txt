`find_till_char`

Waits for another keypress, then moves and
selects forward, stopping before the first
instance of the pressed key. Can take
a count, which will cause it to stop
before the nth instance of the keypress,
rather than the first.

--- Examples ---

The cursor moves forward, stopping before 'c'
and selecting everything along the way.
┌───────────────────────┐  c  ┌───────────────────────┐
│ This i[s] a sentence. │ --> │ This i(s a senten]ce. │
└───────────────────────┘     └───────────────────────┘

The cursor is not stopped by line breaks.
┌───────────────────────────┐     ┌────────────────────────────┐
│ This is the fi[r]st line. │  Q  │ This is the fi(rst line.   │
│ This second line has a Q. │ --> │ This second line has a ]Q. │
└───────────────────────────┘     └────────────────────────────┘
