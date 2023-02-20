
| Crate        | Description                                                      |
| -----------  | -----------                                                      |
| helix-core   | Core editing primitives, functional.                             |
| helix-lsp    | Language server client                                           |
| helix-dap    | Debug Adapter Protocol (DAP) client                              |
| helix-loader | Functions for building, fetching, and loading external resources |
| helix-view   | UI abstractions for use in backends, imperative shell.           |
| helix-term   | Terminal UI                                                      |
| helix-tui    | TUI primitives, forked from tui-rs, inspired by Cursive          |


This document contains a high-level overview of Helix internals.

> NOTE: Use `cargo doc --open` for API documentation as well as dependency
> documentation.

## Core

The core contains basic building blocks used to construct the editor. It is
heavily based on [CodeMirror 6](https://codemirror.net/6/docs/). The primitives
are functional: most operations won't modify data in place but instead return
a new copy.

The main data structure used for representing buffers is a `Rope`. We re-export
the excellent [ropey](https://github.com/cessen/ropey) library. Ropes are cheap
to clone, and allow us to easily make snapshots of a text state.

Multiple selections are a core editing primitive. Document selections are
represented by a `Selection`. Each `Range` in the selection consists of a moving
`head` and an immovable `anchor`. A single cursor in the editor is simply
a selection with a single range, with the head and the anchor in the same
position.

Ropes are modified by constructing an OT-like `Transaction`. It represents
a single coherent change to the document and can be applied to the rope.
A transaction can be inverted to produce an undo. Selections and marks can be
mapped over a transaction to translate to a position in the new text state after
applying the transaction.

> NOTE: `Transaction::change`/`Transaction::change_by_selection` is the main
> interface used to generate text edits.

`Syntax` is the interface used to interact with tree-sitter ASTs for syntax
highlighting and other features.

## View

The `view` layer was supposed to be a frontend-agnostic imperative library that
would build on top of `core` to provide the common editor logic. Currently it's
tied to the terminal UI.

A `Document` ties together the `Rope`, `Selection`(s), `Syntax`, document
`History`, language server (etc.) into a comprehensive representation of an open
file.

A `View` represents an open split in the UI. It holds the currently open
document ID and other related state. Views encapsulate the gutter, status line,
diagnostics, and the inner area where the code is displayed.

> NOTE: Multiple views are able to display the same document, so the document
> contains selections for each view. To retrieve, `document.selection()` takes
> a `ViewId`.

`Info` is the autoinfo box that shows hints when awaiting another key with bindings
like `g` and `m`. It is attached to the viewport as a whole.

`Surface` is like a buffer to which widgets draw themselves to, and the
surface is then rendered on the screen on each cycle.

`Rect`s are areas (simply an x and y coordinate with the origin at the
screen top left and then a height and width) which are part of a
`Surface`. They can be used to limit the area to which a `Component` can
render. For example if we wrap a `Markdown` component in a `Popup`
(think the documentation popup with space+k), Markdown's render method
will get a Rect that is the exact size of the popup.

Widgets are called `Component`s internally, and you can see most of them
in `helix-term/src/ui`. Some components like `Popup` and `Overlay` can take
other components as children.

`Layer`s are how multiple components are displayed, and is simply a
`Vec<Component>`. Layers are managed by the `Compositor`. On each top
level render call, the compositor renders each component in the order
they were pushed into the stack. This makes multiple components "layer"
on top of one another. Hence we get a file picker displayed over the
editor, etc.

The `Editor` holds the global state: all the open documents, a tree
representation of all the view splits, the configuration, and a registry of 
language servers. To open or close files, interact with the editor.

## LSP

A language server protocol client.

## Term

The terminal frontend.

The `main` function sets up a new `Application` that runs the event loop.

`commands.rs` is probably the most interesting file. It contains all commands
(actions tied to keybindings). 

`keymap.rs` links commands to key combinations.


## TUI / Term

TODO: document Component and rendering related stuff
