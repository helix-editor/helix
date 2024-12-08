`till_prev_char`

Waits for another keypress, then moves and
selects backward, stopping before the first
instance of the pressed key. Can take
a count, which will cause it to stop
before the nth instance of the keypress,
rather than the first.

--- Examples ---

The cursor moves backward, stopping before 'h'
and selecting everything along the way.
┌───────────────────────┐  h  ┌───────────────────────┐
│ This is a sent[e]nce. │ --> │ Th[is is a sente)nce. │
└───────────────────────┘     └───────────────────────┘

The cursor is not stopped by line breaks.
┌──────────────────────────────────┐     ┌───────────────────────────────────┐
│ There is a Q in this first line. │  Q  │ There is a Q[ in this first line. │
│ This is the se[c]ond line.       │ --> │ This is the sec)ond line.         │
└──────────────────────────────────┘     └───────────────────────────────────┘
