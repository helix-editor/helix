## Using the quicklist

The quicklist is a temporary list of file locations collected from the current picker contents.
It is useful when a picker already shows the locations you want to visit, but you want to keep moving through them after closing the picker.

While a picker is open, press `Ctrl-q` to populate the quicklist with the picker's current matched items.
This does not render a quicklist window yet. It only stores the locations for navigation.

To browse the current quicklist contents, press `Space-q` to open the quicklist picker.

After populating the quicklist, use:

- `]q` to jump to the next quicklist entry
- `[q` to jump to the previous quicklist entry
- `]l` to jump to the next quicklist entry in the current file
- `[l` to jump to the previous quicklist entry in the current file

The quicklist is currently editor-global. Re-populating it from another picker replaces the previous contents.

### Typical workflow

1. Open a picker that shows file locations, such as global search, diagnostics, symbols, or another location-based picker.
2. Filter the picker until it contains the set of locations you want.
3. Press `Ctrl-q` to copy the current matched items into the quicklist.
4. Optionally inspect the collected entries with `Space-q`.
5. Close the picker and move through the collected locations with `]q` / `[q`, or stay inside the current file with `]l` / `[l`.

### Current limitations

The current implementation derives quicklist entries from the picker's preview location data.
That makes it easy to use across multiple pickers, but it may be less precise than activating the picker entry directly.
For example, picker-specific jump behavior that depends on extra metadata can be approximated as a file and line-range jump in the quicklist.
