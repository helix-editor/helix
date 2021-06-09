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
| Home   | move to the start of the line |
| End   | move to the end of the line |
| m   | Jump to matching bracket | 
| PageUp | Move page up |
| PageDown | Move page down |
| ctrl-u | Move half page up |
| ctrl-d | Move half page down |
| ctrl-i | Jump forward on the jumplist TODO: conflicts tab |
| ctrl-o | Jump backward on the jumplist |
| v   | Enter select (extend) mode |
| g   | Enter goto mode |
| :   | Enter command mode |
| z   | Enter view mode |
| ctrl-w | Enter window mode (maybe will be remove for spc w w later) |
| space   | Enter space mode |
| K | Show documentation for the item under the cursor |

### Changes

| Key | Description |
|-----|-----------|
| r   | replace with a character |
| R   | replace with yanked text |
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

### Diagnostics

> NOTE: `[` and `]` will likely contain more pair mappings in the style of
> [vim-unimpaired](https://github.com/tpope/vim-unimpaired)

| Key | Description |
|-----|-----------|
| [d | Go to previous diagnostic |
| ]d | Go to next diagnostic |
| [D | Go to first diagnostic in document |
| ]D | Go to last diagnostic in document |

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
| h   | Go to the start of the line |
| l   | Go to the end of the line |
| s   | Go to first non-whitespace character of the line |
| t   | Go to the top of the screen |
| m   | Go to the middle of the screen |
| b   | Go to the bottom of the screen |
| d   | Go to definition |
| y   | Go to type definition |
| r   | Go to references |
| i   | Go to implementation |

## Object mode

TODO: Mappings for selecting syntax nodes (a superset of `[`).

## Window mode

This layer is similar to vim keybindings as kakoune does not support window.

| Key | Description |
|-----|-------------|
| w, ctrl-w | Switch to next window |
| v, ctrl-v | Vertical right split |
| h, ctrl-h | Horizontal bottom split |
| q, ctrl-q | Close current window |

## Space mode

This layer is a kludge of mappings I had under leader key in neovim.

| Key | Description |
|-----|-----------|
| f   | Open file picker |
| b   | Open buffer picker |
| w   | Enter window mode |
| space   | Keep primary selection TODO: it's here because space mode replaced it |
