`replace`

Waits for another keypress, then replaces all
selected characters with the pressed key.

--- Examples ---

'a' is replaced with 'e'.
┌──────────────────────────┐  e  ┌──────────────────────────┐
│ Do this, th[a]n do that. │ --> │ Do this, th[e]n do that. │
└──────────────────────────┘     └──────────────────────────┘

All instances of ',' are replaced with '.'.
┌──────────────────────────────┐  .  ┌──────────────────────────────┐
│ This sentence continues(,,,] │ --> │ This sentence continues(...] │
└──────────────────────────────┘     └──────────────────────────────┘

All instances of 'a' are replaced with 'e'.
┌──────────────────────────────────┐  e  ┌──────────────────────────────────┐
│ 1, th[a]n 2, th[a]n 3, th[a]n 4. │ --> │ 1, th[e]n 2, th[e]n 3, th[e]n 4. │
└──────────────────────────────────┘     └──────────────────────────────────┘
