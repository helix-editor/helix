# /home/matt/.steel/cogs/helix/configuration.scm
### **register-lsp-notification-handler**
Register a callback to be called on LSP notifications sent from the server -> client
that aren't currently handled by Helix as a built in.

```scheme
(register-lsp-notification-handler lsp-name event-name handler)
```

* lsp-name : string?
* event-name : string?
* function : (-> hash? any?) ;; Function where the first argument is the parameters

# Examples
```
(register-lsp-notification-handler "dart"
                                   "dart/textDocument/publishClosingLabels"
                                   (lambda (args) (displayln args)))
```
### **cursor-shape**
Shape for cursor in each mode

(cursor-shape #:normal (normal 'block)
              #:select (select 'block)
              #:insert (insert 'block))

# Examples

```scheme
(cursor-shape #:normal 'block #:select 'underline #:insert 'bar)
```
### **set-lsp-config!**
Sets the language server config for a specific language server.

```scheme
(set-lsp-config! lsp config)
```
* lsp : string?
* config: hash?

This will overlay the existing configuration, much like the existing
toml definition does.

Available options for the config hash are:
```scheme
(hash "command" "<command>"
      "args" (list "args" ...)
      "environment" (hash "ENV" "VAR" ...)
      "config" (hash ...)
      "timeout" 100 ;; number
      "required-root-patterns" (listof "pattern" ...))

```

# Examples
```
(set-lsp-config! "jdtls"
   (hash "args" (list "-data" "/home/matt/code/java-scratch/workspace")))
```
### **file-picker-kw**
Sets the configuration for the file picker using keywords.

```scheme
(file-picker-kw #:hidden #t
                #:follow-symlinks #t
                #:deduplicate-links #t
                #:parents #t
                #:ignore #t
                #:git-ignore #t
                #:git-exclude #t
                #:git-global #t
                #:max-depth #f) ;; Expects either #f or an int?
```
By default, max depth is `#f` while everything else is an int?

To use this, call this in your `init.scm` or `helix.scm`:

# Examples
```scheme
(file-picker-kw #:hidden #f)
```
### **file-picker**
Sets the configuration for the file picker using var args.

```scheme
(file-picker . args)
```

The args are expected to be something of the value:
```scheme
(-> FilePickerConfiguration? bool?)    
```

These other functions in this module which follow this behavior are all
prefixed `fp-`, and include:

* fp-hidden
* fp-follow-symlinks
* fp-deduplicate-links
* fp-parents
* fp-ignore
* fp-git-ignore
* fp-git-global
* fp-git-exclude
* fp-max-depth

By default, max depth is `#f` while everything else is an int?

To use this, call this in your `init.scm` or `helix.scm`:

# Examples
```scheme
(file-picker (fp-hidden #f) (fp-parents #f))
```
### **soft-wrap-kw**
Sets the configuration for soft wrap using keyword args.

```scheme
(soft-wrap-kw #:enable #f
              #:max-wrap 20
              #:max-indent-retain 40
              #:wrap-indicator "↪"
              #:wrap-at-text-width #f)
```

The options are as follows:

* #:enable:
  Soft wrap lines that exceed viewport width. Default to off
* #:max-wrap:
  Maximum space left free at the end of the line.
  This space is used to wrap text at word boundaries. If that is not possible within this limit
  the word is simply split at the end of the line.

  This is automatically hard-limited to a quarter of the viewport to ensure correct display on small views.

  Default to 20
* #:max-indent-retain
  Maximum number of indentation that can be carried over from the previous line when softwrapping.
  If a line is indented further then this limit it is rendered at the start of the viewport instead.

  This is automatically hard-limited to a quarter of the viewport to ensure correct display on small views.

  Default to 40
* #:wrap-indicator
  Indicator placed at the beginning of softwrapped lines

  Defaults to ↪
* #:wrap-at-text-width
  Softwrap at `text_width` instead of viewport width if it is shorter

# Examples
```scheme
(soft-wrap-kw #:sw-enable #t)
```
### **soft-wrap**
Sets the configuration for soft wrap using var args.

```scheme
(soft-wrap . args)
```

The args are expected to be something of the value:
```scheme
(-> SoftWrapConfiguration? bool?)    
```
The options are as follows:

* sw-enable:
  Soft wrap lines that exceed viewport width. Default to off
* sw-max-wrap:
  Maximum space left free at the end of the line.
  This space is used to wrap text at word boundaries. If that is not possible within this limit
  the word is simply split at the end of the line.

  This is automatically hard-limited to a quarter of the viewport to ensure correct display on small views.

  Default to 20
* sw-max-indent-retain
  Maximum number of indentation that can be carried over from the previous line when softwrapping.
  If a line is indented further then this limit it is rendered at the start of the viewport instead.

  This is automatically hard-limited to a quarter of the viewport to ensure correct display on small views.

  Default to 40
* sw-wrap-indicator
  Indicator placed at the beginning of softwrapped lines

  Defaults to ↪
* sw-wrap-at-text-width
  Softwrap at `text_width` instead of viewport width if it is shorter

# Examples
```scheme
(soft-wrap (sw-enable #t))
```
### **scrolloff**
Padding to keep between the edge of the screen and the cursor when scrolling. Defaults to 5.
### **scroll_lines**
Number of lines to scroll at once. Defaults to 3
### **mouse**
Mouse support. Defaults to true.
### **shell**
Shell to use for shell commands. Defaults to ["cmd", "/C"] on Windows and ["sh", "-c"] otherwise.
### **line-number**
Line number mode.
### **cursorline**
Highlight the lines cursors are currently on. Defaults to false
### **cursorcolumn**
Highlight the columns cursors are currently on. Defaults to false
### **middle-click-paste**
Middle click paste support. Defaults to true
### **auto-pairs**

Automatic insertion of pairs to parentheses, brackets,
etc. Optionally, this can be a list of 2-tuples to specify a
global list of characters to pair. Defaults to true.
### **auto-completion**
Automatic auto-completion, automatically pop up without user trigger. Defaults to true.
### **auto-format**
Automatic formatting on save. Defaults to true.
### **auto-save**
Automatic save on focus lost and/or after delay.
Time delay in milliseconds since last edit after which auto save timer triggers.
Time delay defaults to false with 3000ms delay. Focus lost defaults to false.
               
### **text-width**
Set a global text_width
### **idle-timeout**
Time in milliseconds since last keypress before idle timers trigger.
Used for various UI timeouts. Defaults to 250ms.
### **completion-timeout**

Time in milliseconds after typing a word character before auto completions
are shown, set to 5 for instant. Defaults to 250ms.
               
### **preview-completion-insert**
Whether to insert the completion suggestion on hover. Defaults to true.
### **completion-trigger-len**
Length to trigger completions
### **completion-replace**
Whether to instruct the LSP to replace the entire word when applying a completion
or to only insert new text
### **auto-info**
Whether to display infoboxes. Defaults to true.
### **true-color**
Set to `true` to override automatic detection of terminal truecolor support in the event of a false negative. Defaults to `false`.
### **insert-final-newline**
Whether to automatically insert a trailing line-ending on write if missing. Defaults to `true`
### **color-modes**
Whether to color modes with different colors. Defaults to `false`.
### **gutters**
Gutter configuration
### **statusline**
Configuration of the statusline elements
### **undercurl**
Set to `true` to override automatic detection of terminal undercurl support in the event of a false negative. Defaults to `false`.
### **search**
Search configuration
### **lsp**
Lsp config
### **terminal**
Terminal config
### **rulers**
Column numbers at which to draw the rulers. Defaults to `[]`, meaning no rulers
### **whitespace**
Whitespace config
### **bufferline**
Persistently display open buffers along the top
### **indent-guides**
Vertical indent width guides
### **workspace-lsp-roots**
Workspace specific lsp ceiling dirs
### **default-line-ending**
Which line ending to choose for new documents. Defaults to `native`. i.e. `crlf` on Windows, otherwise `lf`.
### **smart-tab**
Enables smart tab
### **keybindings**
Keybindings config
### **inline-diagnostics-cursor-line-enable**
Inline diagnostics cursor line
### **inline-diagnostics-end-of-line-enable**
Inline diagnostics end of line
### **get-language-config**
Get the configuration for a specific language
### **set-language-config!**
Set the language configuration
# /home/matt/.steel/cogs/helix/commands.scm
### **quit**
Close the current view.
### **quit!**
Force close the current view, ignoring unsaved changes.
### **open**
Open a file from disk into the current view.
### **buffer-close**
Close the current buffer.
### **buffer-close!**
Close the current buffer forcefully, ignoring unsaved changes.
### **buffer-close-others**
Close all buffers but the currently focused one.
### **buffer-close-others!**
Force close all buffers but the currently focused one.
### **buffer-close-all**
Close all buffers without quitting.
### **buffer-close-all!**
Force close all buffers ignoring unsaved changes without quitting.
### **buffer-next**
Goto next buffer.
### **buffer-previous**
Goto previous buffer.
### **write**
Write changes to disk. Accepts an optional path (:write some/path.txt)
### **write!**
Force write changes to disk creating necessary subdirectories. Accepts an optional path (:write! some/path.txt)
### **write-buffer-close**
Write changes to disk and closes the buffer. Accepts an optional path (:write-buffer-close some/path.txt)
### **write-buffer-close!**
Force write changes to disk creating necessary subdirectories and closes the buffer. Accepts an optional path (:write-buffer-close! some/path.txt)
### **new**
Create a new scratch buffer.
### **format**
Format the file using an external formatter or language server.
### **indent-style**
Set the indentation style for editing. ('t' for tabs or 1-16 for number of spaces.)
### **line-ending**
Set the document's default line ending. Options: crlf, lf.
### **earlier**
Jump back to an earlier point in edit history. Accepts a number of steps or a time span.
### **later**
Jump to a later point in edit history. Accepts a number of steps or a time span.
### **write-quit**
Write changes to disk and close the current view. Accepts an optional path (:wq some/path.txt)
### **write-quit!**
Write changes to disk and close the current view forcefully. Accepts an optional path (:wq! some/path.txt)
### **write-all**
Write changes from all buffers to disk.
### **write-all!**
Forcefully write changes from all buffers to disk creating necessary subdirectories.
### **write-quit-all**
Write changes from all buffers to disk and close all views.
### **write-quit-all!**
Write changes from all buffers to disk and close all views forcefully (ignoring unsaved changes).
### **quit-all**
Close all views.
### **quit-all!**
Force close all views ignoring unsaved changes.
### **cquit**
Quit with exit code (default 1). Accepts an optional integer exit code (:cq 2).
### **cquit!**
Force quit with exit code (default 1) ignoring unsaved changes. Accepts an optional integer exit code (:cq! 2).
### **theme**
Change the editor theme (show current theme if no name specified).
### **yank-join**
Yank joined selections. A separator can be provided as first argument. Default value is newline.
### **clipboard-yank**
Yank main selection into system clipboard.
### **clipboard-yank-join**
Yank joined selections into system clipboard. A separator can be provided as first argument. Default value is newline.
### **primary-clipboard-yank**
Yank main selection into system primary clipboard.
### **primary-clipboard-yank-join**
Yank joined selections into system primary clipboard. A separator can be provided as first argument. Default value is newline.
### **clipboard-paste-after**
Paste system clipboard after selections.
### **clipboard-paste-before**
Paste system clipboard before selections.
### **clipboard-paste-replace**
Replace selections with content of system clipboard.
### **primary-clipboard-paste-after**
Paste primary clipboard after selections.
### **primary-clipboard-paste-before**
Paste primary clipboard before selections.
### **primary-clipboard-paste-replace**
Replace selections with content of system primary clipboard.
### **show-clipboard-provider**
Show clipboard provider name in status bar.
### **change-current-directory**
Change the current working directory.
### **show-directory**
Show the current working directory.
### **encoding**
Set encoding. Based on `https://encoding.spec.whatwg.org`.
### **character-info**
Get info about the character under the primary cursor.
### **reload**
Discard changes and reload from the source file.
### **reload-all**
Discard changes and reload all documents from the source files.
### **update**
Write changes only if the file has been modified.
### **lsp-workspace-command**
Open workspace command picker
### **lsp-restart**
Restarts the given language servers, or all language servers that are used by the current file if no arguments are supplied
### **lsp-stop**
Stops the given language servers, or all language servers that are used by the current file if no arguments are supplied
### **tree-sitter-scopes**
Display tree sitter scopes, primarily for theming and development.
### **tree-sitter-highlight-name**
Display name of tree-sitter highlight scope under the cursor.
### **debug-start**
Start a debug session from a given template with given parameters.
### **debug-remote**
Connect to a debug adapter by TCP address and start a debugging session from a given template with given parameters.
### **debug-eval**
Evaluate expression in current debug context.
### **vsplit**
Open the file in a vertical split.
### **vsplit-new**
Open a scratch buffer in a vertical split.
### **hsplit**
Open the file in a horizontal split.
### **hsplit-new**
Open a scratch buffer in a horizontal split.
### **tutor**
Open the tutorial.
### **goto**
Goto line number.
### **set-language**
Set the language of current buffer (show current language if no value specified).
### **set-option**
Set a config option at runtime.
For example to disable smart case search, use `:set search.smart-case false`.
### **toggle-option**
Toggle a config option at runtime.
For example to toggle smart case search, use `:toggle search.smart-case`.
### **get-option**
Get the current value of a config option.
### **sort**
Sort ranges in selection.
### **reflow**
Hard-wrap the current selection of lines to a given width.
### **tree-sitter-subtree**
Display the smallest tree-sitter subtree that spans the primary selection, primarily for debugging queries.
### **config-reload**
Refresh user config.
### **config-open**
Open the user config.toml file.
### **config-open-workspace**
Open the workspace config.toml file.
### **log-open**
Open the helix log file.
### **insert-output**
Run shell command, inserting output before each selection.
### **append-output**
Run shell command, appending output after each selection.
### **pipe**
Pipe each selection to the shell command.
### **pipe-to**
Pipe each selection to the shell command, ignoring output.
### **run-shell-command**
Run a shell command
### **reset-diff-change**
Reset the diff change at the cursor position.
### **clear-register**
Clear given register. If no argument is provided, clear all registers.
### **redraw**
Clear and re-render the whole UI
### **move**
Move the current buffer and its corresponding file to a different path
### **yank-diagnostic**
Yank diagnostic(s) under primary cursor to register, or clipboard by default
### **read**
Load a file into buffer
### **echo**
Prints the given arguments to the statusline.
### **noop**
Does nothing.
# /home/matt/.steel/cogs/helix/misc.scm
### **hx.cx->pos**
DEPRECATED: Please use `cursor-position`
### **cursor-position**
Returns the cursor position within the current buffer as an integer
### **hx.custom-insert-newline**
DEPRECATED: Please use `insert-newline-hook`
### **insert-newline-hook**
Inserts a new line with the provided indentation.

```scheme
(insert-newline-hook indent-string)
```

indent-string : string?

### **push-component!**

Push a component on to the top of the stack.

```scheme
(push-component! component)
```

component : WrappedDynComponent?
       
### **pop-last-component!**
DEPRECATED: Please use `pop-last-component-by-name!`
### **pop-last-component-by-name!**
Pops the last component off of the stack by name. In other words,
it removes the component matching this name from the stack.

```scheme
(pop-last-component-by-name! name)
```

name : string?
       
### **enqueue-thread-local-callback**

Enqueue a function to be run following this context of execution. This could
be useful for yielding back to the editor in the event you want updates to happen
before your function is run.

```scheme
(enqueue-thread-local-callback callback)
```

callback : (-> any?)
   Function with no arguments.

# Examples

```scheme
(enqueue-thread-local-callback (lambda () (theme "focus_nova")))
```
       
### **set-status!**
Sets the content of the status line
### **send-lsp-command**
Send an lsp command. The `lsp-name` must correspond to an active lsp.
The method name corresponds to the method name that you'd expect to see
with the lsp, and the params can be passed as a hash table. The callback
provided will be called with whatever result is returned from the LSP,
deserialized from json to a steel value.

# Example
```scheme
(define (view-crate-graph)
  (send-lsp-command "rust-analyzer"
                    "rust-analyzer/viewCrateGraph"
                    (hash "full" #f)
                    ;; Callback to run with the result
                    (lambda (result) (displayln result))))
```
### **acquire-context-lock**

Schedule a function to run on the main thread. This is a fairly low level function, and odds are
you'll want to use some abstractions on top of this.

The provided function will get enqueued to run on the main thread, and during the duration of the functions
execution, the provided mutex will be locked.

```scheme
(acquire-context-lock callback-fn mutex)
```

callback-fn : (-> void?)
   Function with no arguments

mutex : mutex?
### **enqueue-thread-local-callback-with-delay**

Enqueue a function to be run following this context of execution, after a delay. This could
be useful for yielding back to the editor in the event you want updates to happen
before your function is run.

```scheme
(enqueue-thread-local-callback-with-delay delay callback)
```

delay : int?
   Time to delay the callback by in milliseconds

callback : (-> any?)
   Function with no arguments.

# Examples

```scheme
(enqueue-thread-local-callback-with-delay 1000 (lambda () (theme "focus_nova"))) ;; Run after 1 second
``
       
### **helix-await-callback**
DEPRECATED: Please use `await-callback`
### **await-callback**

Await the given value, and call the callback function on once the future is completed.

```scheme
(await-callback future callback)
```

* future : future?
* callback (-> any?)
   Function with no arguments
### **add-inlay-hint**

Warning: this is experimental

Adds an inlay hint at the given character index.

```scheme
(add-inlay-hint char-index completion)
```

char-index : int?
completion : string?

### **remove-inlay-hint**

Warning: this is experimental

Removes an inlay hint at the given character index. Note - to remove
properly, the completion must match what was already there.

```scheme
(remove-inlay-hint char-index completion)
```

char-index : int?
completion : string?

# /home/matt/.steel/cogs/helix/editor.scm
### **editor-focus**

Get the current focus of the editor, as a `ViewId`.

```scheme
(editor-focus) -> ViewId
```
       
### **editor-mode**

Get the current mode of the editor

```scheme
(editor-mode) -> Mode?
```
       
### **cx->themes**
DEPRECATED: Please use `themes->list`
### **themes->list**

Get the current themes as a list of strings.

```scheme
(themes->list) -> (listof string?)
```
       
### **editor-all-documents**

Get a list of all of the document ids that are currently open.

```scheme
(editor-all-documents) -> (listof DocumentId?)
```
       
### **cx->cursor**
DEPRECATED: Please use `current-cursor`
### **current-cursor**
Gets the primary cursor position in screen coordinates,
or `#false` if the primary cursor is not visible on screen.

```scheme
(current-cursor) -> (listof? (or Position? #false) CursorKind)
```
       
### **editor-focused-buffer-area**

Get the `Rect` associated with the currently focused buffer.

```scheme
(editor-focused-buffer-area) -> (or Rect? #false)
```
       
### **editor->doc-id**
Get the document from a given view.
### **editor-switch!**
Open the document in a vertical split.
### **editor-set-focus!**
Set focus on the view.
### **editor-set-mode!**
Set the editor mode.
### **editor-doc-in-view?**
Check whether the current view contains a document.
### **set-scratch-buffer-name!**
Set the name of a scratch buffer.
### **set-buffer-uri!**
Set the URI of the buffer
### **editor-doc-exists?**
Check if a document exists.
### **editor-document-last-saved**
Check when a document was last saved (returns a `SystemTime`)
### **editor-document-dirty?**
Check if a document has unsaved changes
### **editor->text**
Get the document as a rope.
### **editor-document->path**
Get the path to a document.
### **register->value**
Get register value as a list of strings.
### **set-editor-clip-top!**
Set the editor clipping at the top.
### **set-editor-clip-right!**
Set the editor clipping at the right.
### **set-editor-clip-left!**
Set the editor clipping at the left.
### **set-editor-clip-bottom!**
Set the editor clipping at the bottom.
# /home/matt/.steel/cogs/helix/themes.scm
### **register-theme**
Register this theme with helix for use
### **attribute**
Class attributes, HTML tag attributes
### **type**
Types
### **type.builtin**
Primitive types provided by the language (`int`, `usize`)
### **type.parameter**
Generic type parameters (`T`)
### **type.enum**
Enum usage
### **type.enum.variant**
Enum variant
### **constructor**
Constructor usage
### **constant**
Constants usage
### **constant.builtin**
Special constants provided by the language (`true`, `false`, `nil`, etc)
### **constant.builtin.boolean**
A special case for highlighting individual booleans
### **constant.character**
Character usage
### **constant.character.escape**
Highlighting individual escape characters
### **constant.numeric**
Numbers
### **constant.numeric.integer**
Integers
### **constant.numeric.float**
Floats
### **string**
Highlighting strings
### **string.regexp**
Highlighting regular expressions
### **string.special**
Special strings
### **string.special.path**
Highlighting paths
### **string.special.url**
Highlighting URLs
### **string.special.symbol**
Erlang/Elixir atoms, Ruby symbols, Clojure keywords
### **comment**
Highlighting comments
### **comment.line**
Single line comments (`//`)
### **comment.block**
Block comments (`/* */`)
### **comment.block.documentation**
Documentation comments (e.g. `///` in Rust)
### **variable**
Variables
### **variable.builtin**
Reserved language variables (`self`, `this`, `super`, etc.)
### **variable.parameter**
Function parameters
### **variable.other**
Other variables
### **variable.other.member**
Fields of composite data types (e.g. structs, unions)
### **variable.other.member.private**
Private fields that use a unique syntax (currently just EMCAScript-based languages)
### **label**
Highlighting labels
### **punctuation**
Highlighting punctuation
### **punctuation.delimiter**
Commas, colon
### **punctuation.bracket**
Parentheses, angle brackets, etc.
### **punctuation.special**
String interpolation brackets
### **keyword**
Highlighting keywords
### **keyword.control**
Control keywords
### **keyword.control.conditional**
if, else
### **keyword.control.repeat**
for, while, loop
### **keyword.control.import**
import, export
### **keyword.control.return**
return keyword
### **keyword.control.exception**
exception keyword
### **keyword.operator**
or, in
### **keyword.directive**
Preprocessor directives (`#if` in C)
### **keyword.function**
fn, func
### **keyword.storage**
Keywords describing how things are stored
### **keyword.storage.type**
The type of something, `class`, `function`, `var`, `let`, etc
### **keyword.storage.modifier**
Storage modifiers like `static`, `mut`, `const`, `ref`, etc
### **operator**
Operators such as `||`, `+=`, `>`, etc
### **function**
Highlighting function calls
### **function.builtin**
Builtin functions
### **function.method**
Calling methods
### **function.method.private**
Private methods that use a unique syntax (currently just ECMAScript-based languages)
### **function.macro**
Highlighting macros
### **function.special**
Preprocessor in C
### **tag**
Tags (e.g. <body> in HTML)
### **tag.builtin**
Builtin tags
### **markup**
Highlighting markdown
### **markup.heading**
Markdown heading
### **markup.heading.marker**
Markdown heading marker
### **markup.heading.marker.1**
Markdown heading text h1
### **markup.heading.marker.2**
Markdown heading text h2
### **markup.heading.marker.3**
Markdown heading text h3
### **markup.heading.marker.4**
Markdown heading text h4
### **markup.heading.marker.5**
Markdown heading text h5
### **markup.heading.marker.6**
Markdown heading text h6
### **markup.list**
Markdown lists
### **markup.list.unnumbered**
Unnumbered markdown lists
### **markup.list.numbered**
Numbered markdown lists
### **markup.list.checked**
Checked markdown lists
### **markup.list.unchecked**
Unchecked markdown lists
### **markup.bold**
Markdown bold
### **markup.italic**
Markdown italics
### **markup.strikethrough**
Markdown strikethrough
### **markup.link**
Markdown links
### **markup.link.url**
URLs pointed to by links
### **markup.link.label**
non-URL link references
### **markup.link.text**
URL and image descriptions in links
### **markup.quote**
Markdown quotes
### **markup.raw**
Markdown raw
### **markup.raw.inline**
Markdown inline raw
### **markup.raw.block**
Markdown raw block
### **diff**
Version control changes
### **diff.plus**
Version control additions
### **diff.plus.gutter**
Version control addition gutter indicator
### **diff.minus**
Version control deletions
### **diff.minus.gutter**
Version control deletion gutter indicator
### **diff.delta**
Version control modifications
### **diff.delta.moved**
Renamed or moved files/changes
### **diff.delta.conflict**
Merge conflicts
### **diff.delta.gutter**
Gutter indicator
### **markup.normal.completion**
For completion doc popup UI
### **markup.normal.hover**
For hover popup UI
### **markup.heading.completion**
For completion doc popup UI
### **markup.heading.hover**
For hover popup UI
### **markup.raw.inline.completion**
For completion doc popup UI
### **markup.raw.inline.hover**
For hover popup UI
### **ui.background.separator**
Picker separator below input line
### **ui.cursor.match**
Matching bracket etc.
### **ui.cursor.primary**
Cursor with primary selection
### **ui.debug.breakpoint**
Breakpoint indicator, found in the gutter
### **ui.debug.active**
Indicator for the line at which debugging execution is paused at, found in the gutter
### **ui.gutter**
Gutter
### **ui.gutter.selected**
Gutter for the line the cursor is on
### **ui.highlight.frameline**
Line at which debugging execution is paused at
### **ui.linenr**
Line numbers
### **ui.linenr.selected**
Line number for the line the cursor is on
### **ui.statusline**
Statusline
### **ui.statusline.inactive**
Statusline (unfocused document)
### **ui.statusline.normal**
Statusline mode during normal mode (only if editor.color-modes is enabled)
### **ui.statusline.insert**
Statusline mode during insert mode (only if editor.color-modes is enabled)
### **ui.statusline.select**
Statusline mode during select mode (only if editor.color-modes is enabled)
### **ui.statusline.separator**
Separator character in statusline
### **ui.bufferline**
Style for the buffer line
### **ui.bufferline.active**
Style for the active buffer in buffer line
### **ui.bufferline.background**
Style for the bufferline background
### **ui.popup**
Documentation popups (e.g. Space + k)
### **ui.popup.info**
Prompt for multiple key options
### **ui.window**
Borderline separating splits
### **ui.help**
Description box for commands
### **ui.text**
Default text style, command prompts, popup text, etc.
### **ui.text.focus**
The currently selected line in the picker
### **ui.text.inactive**
Same as ui.text but when the text is inactive (e.g. suggestions)
### **ui.text.info**
The key: command text in ui.popup.info boxes
### **ui.virtual.ruler**
Ruler columns (see the editor.rules config)
### **ui.virtual.whitespace**
Visible whitespace characters
### **ui.virtual.indent-guide**
Vertical indent width guides
### **ui.virtual.inlay-hint**
Default style for inlay hints of all kinds
### **ui.virtual.inlay-hint.parameter**
Style for inlay hints of kind `parameter` (LSPs are not rquired to set a kind)
### **ui.virtual.inlay-hint.type**
Style for inlay hints of kind `type` (LSPs are not required to set a kind)
### **ui.virtual.wrap**
Soft-wrap indicator (see the editor.soft-wrap config)
### **ui.virtual.jump-label**
Style for virtual jump labels
### **ui.menu**
Code and command completion menus
### **ui.menu.selected**
Selected autocomplete item
### **ui.menu.scroll**
fg sets thumb color, bg sets track color of scrollbar
### **ui.selection**
For selections in the editing area
### **ui.highlight**
Highlighted lines in the picker preview
### **ui.cursorline**
The line of the cursor (if cursorline is enabled)
### **ui.cursorline.primary**
The line of the primary cursor (if cursorline is enabled)
### **ui.cursorline.secondary**
The line of the secondary cursor (if cursorline is enabled)
### **ui.cursorcolumn.primary**
The column of the primary cursor (if cursorcolumn is enabled)
### **ui.cursorcolumn.secondary**
The column of the secondary cursor (if cursorcolumn is enabled)
### **warning**
Diagnostics warning (gutter)
### **error**
Diagnostics error (gutter)
### **info**
Diagnostics info (gutter)
### **hint**
Diagnostics hint (gutter)
### **diagnostic**
Diagnostics fallback style (editing area)
### **diagnostic.hint**
Diagnostics hint (editing area)
### **diagnostic.info**
Diagnostics info (editing area)
### **diagnostic.warning**
Diagnostics warning (editing area)
### **diagnostic.error**
Diagnostics error (editing area)
### **diagnostic.unnecessary**
Diagnostics with unnecessary tag (editing area)
### **diagnostic.deprecated**
Diagnostics with deprecated tag (editing area)
# /home/matt/.steel/cogs/helix/static.scm
### **no_op**
Do nothing
### **move_char_left**
Move left
### **move_char_right**
Move right
### **move_line_up**
Move up
### **move_line_down**
Move down
### **move_visual_line_up**
Move up
### **move_visual_line_down**
Move down
### **extend_char_left**
Extend left
### **extend_char_right**
Extend right
### **extend_line_up**
Extend up
### **extend_line_down**
Extend down
### **extend_visual_line_up**
Extend up
### **extend_visual_line_down**
Extend down
### **copy_selection_on_next_line**
Copy selection on next line
### **copy_selection_on_prev_line**
Copy selection on previous line
### **move_next_word_start**
Move to start of next word
### **move_prev_word_start**
Move to start of previous word
### **move_next_word_end**
Move to end of next word
### **move_prev_word_end**
Move to end of previous word
### **move_next_long_word_start**
Move to start of next long word
### **move_prev_long_word_start**
Move to start of previous long word
### **move_next_long_word_end**
Move to end of next long word
### **move_prev_long_word_end**
Move to end of previous long word
### **move_next_sub_word_start**
Move to start of next sub word
### **move_prev_sub_word_start**
Move to start of previous sub word
### **move_next_sub_word_end**
Move to end of next sub word
### **move_prev_sub_word_end**
Move to end of previous sub word
### **move_parent_node_end**
Move to end of the parent node
### **move_parent_node_start**
Move to beginning of the parent node
### **extend_next_word_start**
Extend to start of next word
### **extend_prev_word_start**
Extend to start of previous word
### **extend_next_word_end**
Extend to end of next word
### **extend_prev_word_end**
Extend to end of previous word
### **extend_next_long_word_start**
Extend to start of next long word
### **extend_prev_long_word_start**
Extend to start of previous long word
### **extend_next_long_word_end**
Extend to end of next long word
### **extend_prev_long_word_end**
Extend to end of prev long word
### **extend_next_sub_word_start**
Extend to start of next sub word
### **extend_prev_sub_word_start**
Extend to start of previous sub word
### **extend_next_sub_word_end**
Extend to end of next sub word
### **extend_prev_sub_word_end**
Extend to end of prev sub word
### **extend_parent_node_end**
Extend to end of the parent node
### **extend_parent_node_start**
Extend to beginning of the parent node
### **find_till_char**
Move till next occurrence of char
### **find_next_char**
Move to next occurrence of char
### **extend_till_char**
Extend till next occurrence of char
### **extend_next_char**
Extend to next occurrence of char
### **till_prev_char**
Move till previous occurrence of char
### **find_prev_char**
Move to previous occurrence of char
### **extend_till_prev_char**
Extend till previous occurrence of char
### **extend_prev_char**
Extend to previous occurrence of char
### **repeat_last_motion**
Repeat last motion
### **replace**
Replace with new char
### **switch_case**
Switch (toggle) case
### **switch_to_uppercase**
Switch to uppercase
### **switch_to_lowercase**
Switch to lowercase
### **page_up**
Move page up
### **page_down**
Move page down
### **half_page_up**
Move half page up
### **half_page_down**
Move half page down
### **page_cursor_up**
Move page and cursor up
### **page_cursor_down**
Move page and cursor down
### **page_cursor_half_up**
Move page and cursor half up
### **page_cursor_half_down**
Move page and cursor half down
### **select_all**
Select whole document
### **select_regex**
Select all regex matches inside selections
### **split_selection**
Split selections on regex matches
### **split_selection_on_newline**
Split selection on newlines
### **merge_selections**
Merge selections
### **merge_consecutive_selections**
Merge consecutive selections
### **search**
Search for regex pattern
### **rsearch**
Reverse search for regex pattern
### **search_next**
Select next search match
### **search_prev**
Select previous search match
### **extend_search_next**
Add next search match to selection
### **extend_search_prev**
Add previous search match to selection
### **search_selection**
Use current selection as search pattern
### **search_selection_detect_word_boundaries**
Use current selection as the search pattern, automatically wrapping with `\b` on word boundaries
### **make_search_word_bounded**
Modify current search to make it word bounded
### **global_search**
Global search in workspace folder
### **extend_line**
Select current line, if already selected, extend to another line based on the anchor
### **extend_line_below**
Select current line, if already selected, extend to next line
### **extend_line_above**
Select current line, if already selected, extend to previous line
### **select_line_above**
Select current line, if already selected, extend or shrink line above based on the anchor
### **select_line_below**
Select current line, if already selected, extend or shrink line below based on the anchor
### **extend_to_line_bounds**
Extend selection to line bounds
### **shrink_to_line_bounds**
Shrink selection to line bounds
### **delete_selection**
Delete selection
### **delete_selection_noyank**
Delete selection without yanking
### **change_selection**
Change selection
### **change_selection_noyank**
Change selection without yanking
### **collapse_selection**
Collapse selection into single cursor
### **flip_selections**
Flip selection cursor and anchor
### **ensure_selections_forward**
Ensure all selections face forward
### **insert_mode**
Insert before selection
### **append_mode**
Append after selection
### **command_mode**
Enter command mode
### **file_picker**
Open file picker
### **file_picker_in_current_buffer_directory**
Open file picker at current buffer's directory
### **file_picker_in_current_directory**
Open file picker at current working directory
### **file_explorer**
Open file explorer in workspace root
### **file_explorer_in_current_buffer_directory**
Open file explorer at current buffer's directory
### **file_explorer_in_current_directory**
Open file explorer at current working directory
### **code_action**
Perform code action
### **buffer_picker**
Open buffer picker
### **jumplist_picker**
Open jumplist picker
### **symbol_picker**
Open symbol picker
### **changed_file_picker**
Open changed file picker
### **select_references_to_symbol_under_cursor**
Select symbol references
### **workspace_symbol_picker**
Open workspace symbol picker
### **diagnostics_picker**
Open diagnostic picker
### **workspace_diagnostics_picker**
Open workspace diagnostic picker
### **last_picker**
Open last picker
### **insert_at_line_start**
Insert at start of line
### **insert_at_line_end**
Insert at end of line
### **open_below**
Open new line below selection
### **open_above**
Open new line above selection
### **normal_mode**
Enter normal mode
### **select_mode**
Enter selection extend mode
### **exit_select_mode**
Exit selection mode
### **goto_definition**
Goto definition
### **goto_declaration**
Goto declaration
### **add_newline_above**
Add newline above
### **add_newline_below**
Add newline below
### **goto_type_definition**
Goto type definition
### **goto_implementation**
Goto implementation
### **goto_file_start**
Goto line number <n> else file start
### **goto_file_end**
Goto file end
### **extend_to_file_start**
Extend to line number<n> else file start
### **extend_to_file_end**
Extend to file end
### **goto_file**
Goto files/URLs in selections
### **goto_file_hsplit**
Goto files in selections (hsplit)
### **goto_file_vsplit**
Goto files in selections (vsplit)
### **goto_reference**
Goto references
### **goto_window_top**
Goto window top
### **goto_window_center**
Goto window center
### **goto_window_bottom**
Goto window bottom
### **goto_last_accessed_file**
Goto last accessed file
### **goto_last_modified_file**
Goto last modified file
### **goto_last_modification**
Goto last modification
### **goto_line**
Goto line
### **goto_last_line**
Goto last line
### **extend_to_last_line**
Extend to last line
### **goto_first_diag**
Goto first diagnostic
### **goto_last_diag**
Goto last diagnostic
### **goto_next_diag**
Goto next diagnostic
### **goto_prev_diag**
Goto previous diagnostic
### **goto_next_change**
Goto next change
### **goto_prev_change**
Goto previous change
### **goto_first_change**
Goto first change
### **goto_last_change**
Goto last change
### **goto_line_start**
Goto line start
### **goto_line_end**
Goto line end
### **goto_column**
Goto column
### **extend_to_column**
Extend to column
### **goto_next_buffer**
Goto next buffer
### **goto_previous_buffer**
Goto previous buffer
### **goto_line_end_newline**
Goto newline at line end
### **goto_first_nonwhitespace**
Goto first non-blank in line
### **trim_selections**
Trim whitespace from selections
### **extend_to_line_start**
Extend to line start
### **extend_to_first_nonwhitespace**
Extend to first non-blank in line
### **extend_to_line_end**
Extend to line end
### **extend_to_line_end_newline**
Extend to line end
### **signature_help**
Show signature help
### **smart_tab**
Insert tab if all cursors have all whitespace to their left; otherwise, run a separate command.
### **insert_tab**
Insert tab char
### **insert_newline**
Insert newline char
### **delete_char_backward**
Delete previous char
### **delete_char_forward**
Delete next char
### **delete_word_backward**
Delete previous word
### **delete_word_forward**
Delete next word
### **kill_to_line_start**
Delete till start of line
### **kill_to_line_end**
Delete till end of line
### **undo**
Undo change
### **redo**
Redo change
### **earlier**
Move backward in history
### **later**
Move forward in history
### **commit_undo_checkpoint**
Commit changes to new checkpoint
### **yank**
Yank selection
### **yank_to_clipboard**
Yank selections to clipboard
### **yank_to_primary_clipboard**
Yank selections to primary clipboard
### **yank_joined**
Join and yank selections
### **yank_joined_to_clipboard**
Join and yank selections to clipboard
### **yank_main_selection_to_clipboard**
Yank main selection to clipboard
### **yank_joined_to_primary_clipboard**
Join and yank selections to primary clipboard
### **yank_main_selection_to_primary_clipboard**
Yank main selection to primary clipboard
### **replace_with_yanked**
Replace with yanked text
### **replace_selections_with_clipboard**
Replace selections by clipboard content
### **replace_selections_with_primary_clipboard**
Replace selections by primary clipboard
### **paste_after**
Paste after selection
### **paste_before**
Paste before selection
### **paste_clipboard_after**
Paste clipboard after selections
### **paste_clipboard_before**
Paste clipboard before selections
### **paste_primary_clipboard_after**
Paste primary clipboard after selections
### **paste_primary_clipboard_before**
Paste primary clipboard before selections
### **indent**
Indent selection
### **unindent**
Unindent selection
### **format_selections**
Format selection
### **join_selections**
Join lines inside selection
### **join_selections_space**
Join lines inside selection and select spaces
### **keep_selections**
Keep selections matching regex
### **remove_selections**
Remove selections matching regex
### **align_selections**
Align selections in column
### **keep_primary_selection**
Keep primary selection
### **remove_primary_selection**
Remove primary selection
### **completion**
Invoke completion popup
### **hover**
Show docs for item under cursor
### **toggle_comments**
Comment/uncomment selections
### **toggle_line_comments**
Line comment/uncomment selections
### **toggle_block_comments**
Block comment/uncomment selections
### **rotate_selections_forward**
Rotate selections forward
### **rotate_selections_backward**
Rotate selections backward
### **rotate_selection_contents_forward**
Rotate selection contents forward
### **rotate_selection_contents_backward**
Rotate selections contents backward
### **reverse_selection_contents**
Reverse selections contents
### **expand_selection**
Expand selection to parent syntax node
### **shrink_selection**
Shrink selection to previously expanded syntax node
### **select_next_sibling**
Select next sibling in the syntax tree
### **select_prev_sibling**
Select previous sibling the in syntax tree
### **select_all_siblings**
Select all siblings of the current node
### **select_all_children**
Select all children of the current node
### **jump_forward**
Jump forward on jumplist
### **jump_backward**
Jump backward on jumplist
### **save_selection**
Save current selection to jumplist
### **jump_view_right**
Jump to right split
### **jump_view_left**
Jump to left split
### **jump_view_up**
Jump to split above
### **jump_view_down**
Jump to split below
### **swap_view_right**
Swap with right split
### **swap_view_left**
Swap with left split
### **swap_view_up**
Swap with split above
### **swap_view_down**
Swap with split below
### **transpose_view**
Transpose splits
### **rotate_view**
Goto next window
### **rotate_view_reverse**
Goto previous window
### **hsplit**
Horizontal bottom split
### **hsplit_new**
Horizontal bottom split scratch buffer
### **vsplit**
Vertical right split
### **vsplit_new**
Vertical right split scratch buffer
### **wclose**
Close window
### **wonly**
Close windows except current
### **select_register**
Select register
### **insert_register**
Insert register
### **copy_between_registers**
Copy between two registers
### **align_view_middle**
Align view middle
### **align_view_top**
Align view top
### **align_view_center**
Align view center
### **align_view_bottom**
Align view bottom
### **scroll_up**
Scroll view up
### **scroll_down**
Scroll view down
### **match_brackets**
Goto matching bracket
### **surround_add**
Surround add
### **surround_replace**
Surround replace
### **surround_delete**
Surround delete
### **select_textobject_around**
Select around object
### **select_textobject_inner**
Select inside object
### **goto_next_function**
Goto next function
### **goto_prev_function**
Goto previous function
### **goto_next_class**
Goto next type definition
### **goto_prev_class**
Goto previous type definition
### **goto_next_parameter**
Goto next parameter
### **goto_prev_parameter**
Goto previous parameter
### **goto_next_comment**
Goto next comment
### **goto_prev_comment**
Goto previous comment
### **goto_next_test**
Goto next test
### **goto_prev_test**
Goto previous test
### **goto_next_entry**
Goto next pairing
### **goto_prev_entry**
Goto previous pairing
### **goto_next_paragraph**
Goto next paragraph
### **goto_prev_paragraph**
Goto previous paragraph
### **dap_launch**
Launch debug target
### **dap_restart**
Restart debugging session
### **dap_toggle_breakpoint**
Toggle breakpoint
### **dap_continue**
Continue program execution
### **dap_pause**
Pause program execution
### **dap_step_in**
Step in
### **dap_step_out**
Step out
### **dap_next**
Step to next
### **dap_variables**
List variables
### **dap_terminate**
End debug session
### **dap_edit_condition**
Edit breakpoint condition on current line
### **dap_edit_log**
Edit breakpoint log message on current line
### **dap_switch_thread**
Switch current thread
### **dap_switch_stack_frame**
Switch stack frame
### **dap_enable_exceptions**
Enable exception breakpoints
### **dap_disable_exceptions**
Disable exception breakpoints
### **shell_pipe**
Pipe selections through shell command
### **shell_pipe_to**
Pipe selections into shell command ignoring output
### **shell_insert_output**
Insert shell command output before selections
### **shell_append_output**
Append shell command output after selections
### **shell_keep_pipe**
Filter selections with shell predicate
### **suspend**
Suspend and return to shell
### **rename_symbol**
Rename symbol
### **increment**
Increment item under cursor
### **decrement**
Decrement item under cursor
### **record_macro**
Record macro
### **replay_macro**
Replay macro
### **command_palette**
Open command palette
### **goto_word**
Jump to a two-character label
### **extend_to_word**
Extend to a two-character label
### **goto_next_tabstop**
Goto next snippet placeholder
### **goto_prev_tabstop**
Goto next snippet placeholder
### **rotate_selections_first**
Make the first selection your primary one
### **rotate_selections_last**
Make the last selection your primary one
### **insert_char**
Insert a given character at the cursor cursor position
### **insert_string**
Insert a given string at the current cursor position
### **set-current-selection-object!**
Update the selection object to the current selection within the editor
### **regex-selection**
Run the given regex within the existing buffer
### **replace-selection-with**
Replace the existing selection with the given string
### **cx->current-file**
Get the currently focused file path
### **enqueue-expression-in-engine**
Enqueue an expression to run at the top level context, 
       after the existing function context has exited.
### **current_selection**
Returns the current selection as a string
### **load-buffer!**
Evaluates the current buffer
### **current-highlighted-text!**
Returns the currently highlighted text as a string
### **get-current-line-number**
Returns the current line number
### **current-selection-object**
Returns the current selection object
### **get-helix-cwd**
Returns the current working directly that helix is using
### **move-window-far-left**
Moves the current window to the far left
### **move-window-far-right**
Moves the current window to the far right
### **get-helix-scm-path**
Returns the path to the helix.scm file as a string
### **get-init-scm-path**
Returns the path to the init.scm file as a string
# /home/matt/.steel/cogs/helix/ext.scm
### **eval-buffer**
Eval the current buffer, morally equivalent to load-buffer!
### **evalp**
Eval prompt
### **running-on-main-thread?**
Check what the main thread id is, compare to the main thread
### **hx.with-context**
If running on the main thread already, just do nothing.
Check the ID of the engine, and if we're already on the
main thread, just continue as is - i.e. just block. This does
not block on the function if this is running on another thread.

```scheme
(hx.with-context thunk)
```
thunk : (-> any?) ;; Function that has no arguments

# Examples
```scheme
(spawn-native-thread
  (lambda () 
    (hx.with-context (lambda () (theme "nord")))))
```
### **hx.block-on-task**
Block on the given function.
```scheme
(hx.block-on-task thunk)
```
thunk : (-> any?) ;; Function that has no arguments

# Examples
```scheme
(define thread
  (spawn-native-thread
    (lambda () 
      (hx.block-on-task (lambda () (theme "nord") 10)))))

;; Some time later, in a different context - if done at the same time,
;; this will deadline, since the join depends on the callback previously
;; executing.
(equal? (thread-join! thread) 10) ;; => #true
```
# /home/matt/.steel/cogs/helix/components.scm
### **theme->bg**
Gets the `Style` associated with the bg for the current theme
### **theme->fg**
Gets the `style` associated with the fg for the current theme
### **theme-scope**
Get the `Style` associated with the given scope from the current theme
### **Position?**
Check if the given value is a `Position`

```scheme
(Position? value) -> bool?
```

value : any?

       
### **Style?**
Check if the given valuie is `Style`

```scheme
(Style? value) -> bool?
```

value : any?
### **Buffer?**

Checks if the given value is a `Buffer`

```scheme
(Buffer? value) -> bool?
```

value : any?
       
### **buffer-area**

Get the `Rect` associated with the given `Buffer`

```scheme
(buffer-area buffer)
```

* buffer : Buffer?
       
### **frame-set-string!**

Set the string at the given `x` and `y` positions for the given `Buffer`, with a provided `Style`.

```scheme
(frame-set-string! buffer x y string style)
```

buffer : Buffer?,
x : int?,
y : int?,
string: string?,
style: Style?,
       
### **SteelEventResult?**

Check whether the given value is a `SteelEventResult`.

```scheme
(SteelEventResult? value) -> bool?
```

value : any?

       
### **new-component!**

Construct a new dynamic component. This is used for creating widgets or floating windows
that exist outside of the buffer. This just constructs the component, it does not push the component
on to the component stack. For that, you'll use `push-component!`.

```scheme
(new-component! name state render function-map)
```

name : string? - This is the name of the comoponent itself.
state : any? - Typically this is a struct that holds the state of the component.
render : (-> state? Rect? Buffer?)
   This is a function that will get called with each frame. The first argument is the state object provided,
   and the second is the `Rect?` to render against, ultimately against the `Buffer?`.

function-map : (hashof string? function?)
   This is a hashmap of strings -> function that contains a few important functions:

   "handle_event" : (-> state? Event?) -> SteelEventResult?

       This is called on every event with an event object. There are multiple options you can use
       when returning from this function:

       * event-result/consume
       * event-result/consume-without-rerender
       * event-result/ignore
       * event-result/close

       See the associated docs for those to understand the implications for each.
       
   "cursor" : (-> state? Rect?) -> Position?

       This tells helix where to put the cursor.
   
   "required_size": (-> state? (pair? int?)) -> (pair? int?)

       Seldom used: TODO
   
### **position**

Construct a new `Position`.

```scheme
(position row col) -> Position?
```

row : int?
col : int?
       
### **position-row**

Get the row associated with the given `Position`.

```scheme
(position-row pos) -> int?
```

pos : `Position?`
       
### **position-col**

Get the col associated with the given `Position`.

```scheme
(position-col pos) -> int?
```

pos : `Position?`
### **set-position-row!**
Set the row for the given `Position`

```scheme
(set-position-row! pos row)
```

pos : Position?
row : int?
       
### **set-position-col!**
Set the col for the given `Position`

```scheme
(set-position-col! pos col)
```

pos : Position?
col : int?
       
### **Rect?**
Check if the given value is a `Rect`

```scheme
(Rect? value) -> bool?
```

value : any?

       
### **area**

Constructs a new `Rect`.

(area x y width height)

* x : int?
* y : int?
* width: int?
* height: int?

# Examples

```scheme
(area 0 0 100 200)
```
### **area-x**
Get the `x` value of the given `Rect`

```scheme
(area-x area) -> int?
```

area : Rect?
       
### **area-y**
Get the `y` value of the given `Rect`

```scheme
(area-y area) -> int?
```

area : Rect?
       
### **area-width**
Get the `width` value of the given `Rect`

```scheme
(area-width area) -> int?
```

area : Rect?
       
### **area-height**
Get the `height` value of the given `Rect`

```scheme
(area-height area) -> int?
```

area : Rect?
       
### **Widget/list?**
Check whether the given value is a list widget.

```scheme
(Widget/list? value) -> bool?
```

value : any?
       
### **widget/list**
Creates a new `List` widget with the given items.

```scheme
(widget/list lst) -> Widget?
```

* lst : (listof string?)
       
### **widget/list/render**


Render the given `Widget/list` onto the provided `Rect` within the given `Buffer`.

```scheme
(widget/list/render buf area lst)
```

* buf : `Buffer?`
* area : `Rect?`
* lst : `Widget/list?`
       
### **block**
Creates a block with the following styling:

```scheme
(block)
```

* borders - all
* border-style - default style + white fg
* border-type - rounded
* style - default + black bg
       
### **make-block**

Create a `Block` with the provided styling, borders, and border type.


```scheme
(make-block style border-style borders border_type)
```

* style : Style?
* border-style : Style?
* borders : string?
* border-type: String?

Valid border-types include:
* "plain"
* "rounded"
* "double"
* "thick"

Valid borders include:
* "top"
* "left"
* "right"
* "bottom"
* "all"
       
### **block/render**

Render the given `Block` over the given `Rect` onto the provided `Buffer`.

```scheme
(block/render buf area block)
```

buf : Buffer?
area: Rect?
block: Block?
           
       
### **buffer/clear**
Clear a `Rect` in the `Buffer`

```scheme
(buffer/clear area)
```

area : Rect?
       
### **buffer/clear-with**
Clear a `Rect` in the `Buffer` with a default `Style`

```scheme
(buffer/clear-with area style)
```

area : Rect?
style : Style?
       
### **set-color-rgb!**

Mutate the r/g/b of a color in place, to avoid allocation.

```scheme
(set-color-rgb! color r g b)
```

color : Color?
r : int?
g : int?
b : int?
### **set-color-indexed!**

Mutate this color to be an indexed color.

```scheme
(set-color-indexed! color index)
```

color : Color?
index: int?
   
### **Color?**
Check if the given value is a `Color`.

```scheme
(Color? value) -> bool?
```

value : any?

       
### **Color/Reset**

Singleton for the reset color.
       
### **Color/Black**

Singleton for the color black.
       
### **Color/Red**

Singleton for the color red.
       
### **Color/White**

Singleton for the color white.
       
### **Color/Green**

Singleton for the color green.
       
### **Color/Yellow**

Singleton for the color yellow.
       
### **Color/Blue**

Singleton for the color blue.
       
### **Color/Magenta**

Singleton for the color magenta.
       
### **Color/Cyan**

Singleton for the color cyan.
       
### **Color/Gray**

Singleton for the color gray.
       
### **Color/LightRed**

Singleton for the color light read.
       
### **Color/LightGreen**

Singleton for the color light green.
       
### **Color/LightYellow**

Singleton for the color light yellow.
       
### **Color/LightBlue**

Singleton for the color light blue.
       
### **Color/LightMagenta**

Singleton for the color light magenta.
       
### **Color/LightCyan**

Singleton for the color light cyan.
       
### **Color/LightGray**

Singleton for the color light gray.
       
### **Color/rgb**

Construct a new color via rgb.

```scheme
(Color/rgb r g b) -> Color?
```

r : int?
g : int?
b : int?
       
### **Color-red**

Get the red component of the `Color?`.

```scheme
(Color-red color) -> int?
```

color * Color?
       
### **Color-green**

Get the green component of the `Color?`.

```scheme
(Color-green color) -> int?
```

color * Color?
### **Color-blue**

Get the blue component of the `Color?`.

```scheme
(Color-blue color) -> int?
```

color * Color?
### **Color/Indexed**


Construct a new indexed color.

```scheme
(Color/Indexed index) -> Color?
```

* index : int?
       
### **set-style-fg!**


Mutates the given `Style` to have the fg with the provided color.

```scheme
(set-style-fg! style color)
```

style : `Style?`
color : `Color?`
       
### **style-fg**


Constructs a new `Style` with the provided `Color` for the fg.

```scheme
(style-fg style color) -> Style
```

style : Style?
color: Color?
       
### **style-bg**


Constructs a new `Style` with the provided `Color` for the bg.

```scheme
(style-bg style color) -> Style
```

style : Style?
color: Color?
       
### **style-with-italics**


Constructs a new `Style` with italcs.

```scheme
(style-with-italics style) -> Style
```

style : Style?
       
### **style-with-bold**


Constructs a new `Style` with bold styling.

```scheme
(style-with-bold style) -> Style
```

style : Style?
       
### **style-with-dim**


Constructs a new `Style` with dim styling.

```scheme
(style-with-dim style) -> Style
```

style : Style?
       
### **style-with-slow-blink**


Constructs a new `Style` with slow blink.

```scheme
(style-with-slow-blink style) -> Style
```

style : Style?
       
### **style-with-rapid-blink**


Constructs a new `Style` with rapid blink.

```scheme
(style-with-rapid-blink style) -> Style
```

style : Style?
       
### **style-with-reversed**


Constructs a new `Style` with revered styling.

```scheme
(style-with-reversed style) -> Style
```

style : Style?
       
### **style-with-hidden**

Constructs a new `Style` with hidden styling.

```scheme
(style-with-hidden style) -> Style
```

style : Style?
       
### **style-with-crossed-out**


Constructs a new `Style` with crossed out styling.

```scheme
(style-with-crossed-out style) -> Style
```

style : Style?
       
### **style->fg**


Return the color on the style, or #false if not present.

```scheme
(style->fg style) -> (or Color? #false)
```

style : Style?
           
       
### **style->bg**


Return the color on the style, or #false if not present.

```scheme
(style->bg style) -> (or Color? #false)
```

style : Style?
           
       
### **set-style-bg!**


Mutate the background style on the given style to a given color.

```scheme
(set-style-bg! style color)
```

style : Style?
color : Color?
           
       
### **style-underline-color**


Return a new style with the provided underline color.

```scheme
(style-underline-color style color) -> Style?

```
style : Style?
color : Color?
           
       
### **style-underline-style**

Return a new style with the provided underline style.

```scheme
(style-underline-style style underline-style) -> Style?

```

style : Style?
underline-style : UnderlineStyle?

### **UnderlineStyle?**

Check if the provided value is an `UnderlineStyle`.

```scheme
(UnderlineStyle? value) -> bool?

```
value : any?
### **Underline/Reset**

Singleton for resetting the underling style.
       
### **Underline/Line**

Singleton for the line underline style.
       
### **Underline/Curl**

Singleton for the curl underline style.
       
### **Underline/Dotted**

Singleton for the dotted underline style.
       
### **Underline/Dashed**

Singleton for the dashed underline style.
       
### **Underline/DoubleLine**

Singleton for the double line underline style.
       
### **event-result/consume**

Singleton for consuming an event. If this is returned from an event handler, the event
will not continue to be propagated down the component stack. This also will trigger a
re-render.
       
### **event-result/consume-without-rerender**

Singleton for consuming an event. If this is returned from an event handler, the event
will not continue to be propagated down the component stack. This will _not_ trigger
a re-render.
       
### **event-result/ignore**

Singleton for ignoring an event. If this is returned from an event handler, the event
will not continue to be propagated down the component stack. This will _not_ trigger
a re-render.
       
### **event-result/ignore-and-close**

Singleton for ignoring an event. If this is returned from an event handler, the event
will continue to be propagated down the component stack, and the component will be
popped off of the stack and removed.
       
### **event-result/close**

Singleton for consuming an event. If this is returned from an event handler, the event
will not continue to be propagated down the component stack, and the component will
be popped off of the stack and removed.
       
### **style**

Constructs a new default style.

```scheme
(style) -> Style?
```
       
### **Event?**
Check if this value is an `Event`

```scheme
(Event? value) -> bool?
```
value : any?
       
### **key-event?**
Checks if the given event is a key event.

```scheme
(key-event? event) -> bool?
```

* event : Event?
       
### **key-event-char**
Get the character off of the event, if there is one.

```scheme
(key-event-char event) -> (or char? #false)
```
event : Event?
       
### **key-event-modifier**

Get the key event modifier off of the event, if there is one.

```scheme
(key-event-modifier event) -> (or int? #false)
```
event : Event?
       
### **key-modifier-ctrl**

The key modifier bits associated with the ctrl key modifier.
       
### **key-modifier-shift**

The key modifier bits associated with the shift key modifier.
       
### **key-modifier-alt**

The key modifier bits associated with the alt key modifier.
       
### **key-event-F?**
Check if this key event is associated with an `F<x>` key, e.g. F1, F2, etc.

```scheme
(key-event-F? event number) -> bool?
```
event : Event?
number : int?
       
### **mouse-event?**

Check if this event is a mouse event.

```scheme
(mouse-event event) -> bool?
```
event : Event?
### **event-mouse-kind**
Convert the mouse event kind into an integer representing the state.

```scheme
(event-mouse-kind event) -> (or int? #false)
```

event : Event?

This is the current mapping today:

```rust
match kind {
   helix_view::input::MouseEventKind::Down(MouseButton::Left) => 0,
   helix_view::input::MouseEventKind::Down(MouseButton::Right) => 1,
   helix_view::input::MouseEventKind::Down(MouseButton::Middle) => 2,
   helix_view::input::MouseEventKind::Up(MouseButton::Left) => 3,
   helix_view::input::MouseEventKind::Up(MouseButton::Right) => 4,
   helix_view::input::MouseEventKind::Up(MouseButton::Middle) => 5,
   helix_view::input::MouseEventKind::Drag(MouseButton::Left) => 6,
   helix_view::input::MouseEventKind::Drag(MouseButton::Right) => 7,
   helix_view::input::MouseEventKind::Drag(MouseButton::Middle) => 8,
   helix_view::input::MouseEventKind::Moved => 9,
   helix_view::input::MouseEventKind::ScrollDown => 10,
   helix_view::input::MouseEventKind::ScrollUp => 11,
   helix_view::input::MouseEventKind::ScrollLeft => 12,
   helix_view::input::MouseEventKind::ScrollRight => 13,
}
```

Any unhandled event that does not match this will return `#false`.
### **event-mouse-row**


Get the row from the mouse event, of #false if it isn't a mouse event.

```scheme
(event-mouse-row event) -> (or int? #false)
```

event : Event?
           
       
### **event-mouse-col**


Get the col from the mouse event, of #false if it isn't a mouse event.

```scheme
(event-mouse-row event) -> (or int? #false)
```

event : Event?
       
### **mouse-event-within-area?**
Check whether the given mouse event occurred within a given `Rect`.

```scheme
(mouse-event-within-area? event area) -> bool?
```

event : Event?
area : Rect?
       
### **key-event-escape?**

Check whether the given event is the key: escape

```scheme
(key-event-escape? event)
```
event: Event?
### **key-event-backspace?**

Check whether the given event is the key: backspace

```scheme
(key-event-backspace? event)
```
event: Event?
### **key-event-enter?**

Check whether the given event is the key: enter

```scheme
(key-event-enter? event)
```
event: Event?
### **key-event-left?**

Check whether the given event is the key: left

```scheme
(key-event-left? event)
```
event: Event?
### **key-event-right?**

Check whether the given event is the key: right

```scheme
(key-event-right? event)
```
event: Event?
### **key-event-up?**

Check whether the given event is the key: up

```scheme
(key-event-up? event)
```
event: Event?
### **key-event-down?**

Check whether the given event is the key: down

```scheme
(key-event-down? event)
```
event: Event?
### **key-event-home?**

Check whether the given event is the key: home

```scheme
(key-event-home? event)
```
event: Event?
### **key-event-page-up?**

Check whether the given event is the key: page-up

```scheme
(key-event-page-up? event)
```
event: Event?
### **key-event-page-down?**

Check whether the given event is the key: page-down

```scheme
(key-event-page-down? event)
```
event: Event?
### **key-event-tab?**

Check whether the given event is the key: tab

```scheme
(key-event-tab? event)
```
event: Event?
### **key-event-delete?**

Check whether the given event is the key: delete

```scheme
(key-event-delete? event)
```
event: Event?
### **key-event-insert?**

Check whether the given event is the key: insert

```scheme
(key-event-insert? event)
```
event: Event?
### **key-event-null?**

Check whether the given event is the key: null

```scheme
(key-event-null? event)
```
event: Event?
### **key-event-caps-lock?**

Check whether the given event is the key: caps-lock

```scheme
(key-event-caps-lock? event)
```
event: Event?
### **key-event-scroll-lock?**

Check whether the given event is the key: scroll-lock

```scheme
(key-event-scroll-lock? event)
```
event: Event?
### **key-event-num-lock?**

Check whether the given event is the key: num-lock

```scheme
(key-event-num-lock? event)
```
event: Event?
### **key-event-print-screen?**

Check whether the given event is the key: print-screen

```scheme
(key-event-print-screen? event)
```
event: Event?
### **key-event-pause?**

Check whether the given event is the key: pause

```scheme
(key-event-pause? event)
```
event: Event?
### **key-event-menu?**

Check whether the given event is the key: menu

```scheme
(key-event-menu? event)
```
event: Event?
### **key-event-keypad-begin?**

Check whether the given event is the key: keypad-begin

```scheme
(key-event-keypad-begin? event)
```
event: Event?
# helix/core/text
### **Rope?**
Check if the given value is a rope
### **rope->byte-slice**
Take a slice of this rope using byte offsets

```scheme
(rope->byte-slice rope start end) -> Rope?
```

* rope: Rope?
* start: (and positive? int?)
* end: (and positive? int?)
### **rope->line**
Get the line at the given line index. Returns a rope.

```scheme
(rope->line rope index) -> Rope?

```

* rope : Rope?
* index : (and positive? int?)
### **rope->slice**
Take a slice from using character indices from the rope.
Returns a new rope value.

```scheme
(rope->slice rope start end) -> Rope?
```

* rope : Rope?
* start: (and positive? int?)
* end: (and positive? int?)
### **rope->string**
Convert the given rope to a string
### **rope-byte->line**
Convert the given byte offset to a line offset for a given rope

```scheme
(rope-byte->line rope byte-index) -> int?
```

* rope : Rope?
* byte-index : int?

            
### **rope-char->byte**
Convert the byte offset into a character offset for a given rope
### **rope-char->line**
Convert the given character offset to a line offset for a given rope

```scheme
(rope-char->line rope char-index) -> int?
```

* rope : Rope?
* char-index : int?

            
### **rope-char-ref**
Get the character at the given index
### **rope-ends-with?**
Check if the rope ends with a given pattern
### **rope-insert-char**
Insert a character at the given index
### **rope-insert-string**
Insert a string at the given index into the rope
### **rope-len-bytes**
Get the length of the rope in bytes
### **rope-len-chars**
Get the length of the rope in characters
### **rope-len-lines**
Get the number of lines in the rope
### **rope-line->byte**
Convert the given line index to a byte offset for a given rope

```scheme
(rope-line->byte rope line-offset) -> int?
```

* rope : Rope?
* line-offset: int?
            
### **rope-line->char**
Convert the given line index to a character offset for a given rope

```scheme
(rope-line->char rope line-offset) -> int?
```

* rope : Rope?
* line-offset: int?
            
### **rope-starts-with?**
Check if the rope starts with a given pattern
### **rope-trim-start**
Remove the leading whitespace from the given rope
### **string->rope**
Converts a string into a rope.

```scheme
(string->rope value) -> Rope?
```

* value : string?
            
