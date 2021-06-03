# Keymap

## Normal mode

### Movement

| Key | Description |
|-----|-----------|
| h, Left   | move left |
| j, Down   | move down |
| k, Up   | move up |
| l, Right   | move right |
| w   | move next word start |
| b   | move previous word start |
| e   | move next word end |
| t   | find 'till next char |
| f   | find next char |
| T   | find 'till previous char |
| F   | find previous char |
| ^   | move to the start of the line |
| $   | move to the end of the line |
| m   | Jump to matching bracket | 
| PageUp | Move page up |
| PageDown | Move page down |
| ctrl-u | Move half page up |
| ctrl-d | Move half page down |
| Tab | Switch to next view |
| ctrl-i | Jump forward on the jumplist TODO: conflicts tab |
| ctrl-o | Jump backward on the jumplist |
| v   | Enter select (extend) mode |
| g   | Enter goto mode |
| :   | Enter command mode |
| z   | Enter view mode |
| space   | Enter space mode |
| K | Show documentation for the item under the cursor |

### Changes

| Key | Description |
|-----|-----------|
| r   | replace (single character change) |
| i   | Insert before selection |
| a   | Insert after selection (append) |
| I   | Insert at the start of the line | 
| A   | Insert at the end of the line | 
| o   | Open new line below selection | 
| o   | Open new line above selection | 
| u   | Undo change |
| U   | Redo change |
| y   | Yank selection |
| p   | Paste after selection |
| P   | Paste before selection |
| >   | Indent selection |
| <   | Unindent selection |
| =   | Format selection |
| d   | Delete selection | 
| c   | Change selection (delete and enter insert mode) | 

### Selection manipulation

| Key | Description |
|-----|-----------|
| s   | Select all regex matches inside selections | 
| S   | Split selection into subselections on regex matches | 
| alt-s   | Split selection on newlines | 
| ;   | Collapse selection onto a single cursor | 
| alt-;   | Flip selection cursor and anchor | 
| %   | Select entire file | 
| x   | Select current line | 
| X   | Extend to next line | 
| [   | Expand selection to parent syntax node TODO: pick a key | 
| J | join lines inside selection |
| K | keep selections matching the regex TODO: overlapped by hover help |
| space | keep only the primary selection TODO: overlapped by space mode |
| ctrl-c | Comment/uncomment the selections | 

### Search

> TODO: The search implementation isn't ideal yet -- we don't support searching
in reverse, or searching via smartcase.

| Key | Description |
|-----|-----------|
| /   | Search for regex pattern | 
| n   | Select next search match | 
| N   | Add next search match to selection | 
| *   | Use current selection as the search pattern | 

## Select / extend mode

I'm still pondering whether to keep this mode or not. It changes movement
commands to extend the existing selection instead of replacing it.

> NOTE: It's a bit confusing at the moment because extend hasn't been
> implemented for all movement commands yet.

## View mode

View mode is intended for scrolling and manipulating the view without changing
the selection.

| Key | Description |
|-----|-----------|
| z , c | Vertically center the line |
| t   | Align the line to the top of the screen |
| b   | Align the line to the bottom of the screen |
| m   | Align the line to the middle of the screen (horizontally) |
| j   | Scroll the view downwards |
| k   | Scroll the view upwards |

## Goto mode

Jumps to various locations.

> NOTE: Some of these features are only available with the LSP present.

| Key | Description |
|-----|-----------|
| g   | Go to the start of the file |
| e   | Go to the end of the file |
| d   | Go to definition |
| t   | Go to type definition |
| r   | Go to references |
| i   | Go to implementation |

## Object mode

TODO: Mappings for selecting syntax nodes (a superset of `[`).

## Space mode

This layer is a kludge of mappings I had under leader key in neovim.

| Key | Description |
|-----|-----------|
| f   | Open file picker |
| b   | Open buffer picker |
| v   | Open a new vertical split into the current file |
| w   | Save changes to file |
| c   | Close the current split |
| space   | Keep primary selection TODO: it's here because space mode replaced it |
