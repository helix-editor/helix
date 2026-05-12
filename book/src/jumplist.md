## Using the jumplist

To help with quick navigation, Helix maintains a list of "jumps" called the jumplist.
Whenever you make a significant movement (see next section), Helix stores your selections from before the move as a jump.
A jump serves as a kind of checkpoint, allowing you to jump to a separate location, make edits, and return to where you were with your previous selections.
This way, the jumplist tracks both your previous location and your selections.
You can manually save a jump by using `Ctrl-s`.
To jump backward in the jumplist, use `Ctrl-o`; to go forward, use `Ctrl-i`. To view and select from the full jumplist, use `Space-j` to open the jumplist picker.

### What makes a jump
The following is a non-exhaustive list of which actions add a jump to the jumplist:
- Switching buffers
  - Using the buffer picker, going to the next/previous buffer
  - Going to the last accessed/modified file
  - Making a new file (`:new FILE`)
  - Opening a file (`:open FILE`)
    - Includes `:log-open`, `:config-open`, `:config-open-workspace`, `:tutor`
  - Navigating by pickers, global search, or the file explorer
  - `goto_file` (`gf`)
- Big in-file movements
  - `select_regex` (`s`)
  - `split_regex` (`S`)
  - `search` (`/`)
  - `keep_selections` and `remove_selections` (`K` and `<A-K>`)
  - `goto_file_start` (`gg`)
  - `goto_file_end`
  - `goto_last_line` (`ge`)
  - `:goto 123` / `:123` / `123G`
  - `goto_definition` (`gd`)
  - `goto_declaration` (`gD`)
  - `goto_type_definition` (`gy`)
  - `goto_reference` (`gr`)
- Other
  - `Ctrl-s` manually creates a jump
  - Trying to close a modified buffer can switch you to that buffer and create a jump
  - The debugger can create jumps as you jump stack frames
