# Building

You will need:

* A clone of this fork, on the branch `steel-event-system`

## Installing helix

Just run

`cargo xtask steel`

To install the `hx` executable, with steel as a plugin language. This also includes:

The `steel` executable, the steel language server, the steel dylib installer, and the steel package manager `forge`.

## Setting up configurations for helix

There are 2 important files you'll want, which should be auto generated during the installation process if they don't already exist:

* `~/.config/helix/helix.scm`
* `~/.config/helix/init.scm`

Note - these both live inside the same directory that helix sets up for runtime configurations.

### `helix.scm`

The `helix.scm` module will be loaded first before anything else, the runtime will `require` this module, and any functions exported will now be available
to be used as typed commands. For example:


```scheme
# helix.scm
(require "helix/editor.scm")
(require (prefix-in helix. "helix/commands.scm"))
(require (prefix-in helix.static. "helix/static.scm"))

(provide shell git-add open-helix-scm open-init-scm)

;;@doc
;; Specialized shell implementation, where % is a wildcard for the current file
(define (shell cx . args)
  ;; Replace the % with the current file
  (define expanded (map (lambda (x) (if (equal? x "%") (current-path cx) x)) args))
  (apply helix.run-shell-command expanded))

;;@doc
;; Adds the current file to git	
(define (git-add cx)
  (shell cx "git" "add" "%"))

(define (current-path)
  (let* ([focus (editor-focus)]
         [focus-doc-id (editor->doc-id focus)])
    (editor-document->path focus-doc-id)))

;;@doc
;; Open the helix.scm file
(define (open-helix-scm)
  (helix.open (helix.static.get-helix-scm-path)))

;;@doc
;; Opens the init.scm file
(define (open-init-scm)
  (helix.open (helix.static.get-init-scm-path)))
  
	
```

Now, if you'd like to add the current file you're editing to git, simply type `:git-add` - you'll see the doc pop up with it since we've annotated the function
with the `@doc` symbol. Hitting enter will execute the command.

You can also conveniently open the `helix.scm` file by using the typed command `:open-helix-scm`.


### `init.scm`

The `init.scm` file is run at the top level, immediately after the `helix.scm` module is `require`d. The helix context is available here, so you can interact with the editor.

The helix context is bound to the top level variable `*helix.cx*`.

For example, if we wanted to select a random theme at startup:

```scheme
# init.scm

(require-builtin steel/random as rand::)
(require (prefix-in helix. "helix/commands.scm"))
(require (prefix-in helix.static. "helix/static.scm"))

;; Picking one from the possible themes
(define possible-themes '("ayu_mirage" "tokyonight_storm" "catppuccin_macchiato"))

(define (select-random lst)
  (let ([index (rand::rng->gen-range 0 (length lst))]) (list-ref lst index)))

(define (randomly-pick-theme options)
  ;; Randomly select the theme from the possible themes list
  (helix.theme (select-random options)))

(randomly-pick-theme possible-themes)

```

### Libraries for helix

There are a handful of extra libraries in development for extending helix, and can be found here https://github.com/mattwparas/helix-config.

If you'd like to use them, create a directory called `cogs` in your `.config/helix` directory, and copy the files in there.

### options.scm

If you'd like to override configurations from your toml config:


```scheme
# init.scm

(require "helix/configuration.scm")

(file-picker (fp-hidden #f))
(cursorline #t)
(soft-wrap (sw-enable #t))

```


### keymaps.scm

Applying custom keybindings for certain file extensions:

```scheme
# init.scm

(require "cogs/keymaps.scm")
(require (only-in "cogs/file-tree.scm" FILE-TREE-KEYBINDINGS FILE-TREE))
(require (only-in "cogs/recentf.scm" recentf-open-files get-recent-files recentf-snapshot))

;; Set the global keybinding for now
(add-global-keybinding (hash "normal" (hash "C-r" (hash "f" ":recentf-open-files"))))

(define scm-keybindings (hash "insert" (hash "ret" ':scheme-indent "C-l" ':insert-lambda)))

;; Grab whatever the existing keybinding map is
(define standard-keybindings (deep-copy-global-keybindings))

(define file-tree-base (deep-copy-global-keybindings))

(merge-keybindings standard-keybindings scm-keybindings)
(merge-keybindings file-tree-base FILE-TREE-KEYBINDINGS)

(set-global-buffer-or-extension-keymap (hash "scm" standard-keybindings FILE-TREE file-tree-base))
	
```

In insert mode, this overrides the `ret` keybinding to instead use a custom scheme indent function. Functions _must_ be available as typed commands, and are referred to
as symbols. So in this case, the `scheme-indent` function was exported by my `helix.scm` module.


## Writing a plugin

### Getting setup

Before you start, you should make sure that your configuration for the steel lsp is wired up correctly. This will give you
access to the documentation that will help you as you write your plugin. To configure the LSP, you can add this to your
`init.scm`:

```scheme
(require "helix/configuration.scm")
(define-lsp "steel-language-server" (command "steel-language-server") (args '()))
(define-language "scheme"
                 (language-servers '("steel-language-server")))
```

This will give you an interactive setup that can help you run plugins as you go. I also like to evaluate commands
via the buffer, by either typing them in to the command prompt or by loading the current buffer. To load the current
buffer, you can type `:eval-buffer`, or to evaluate an individual command, you can run `:evalp` - note, in your init.scm, you
may need to add:

```scheme
(require (only-in "helix/ext" evalp eval-buffer))
```

This brings those functions to the top level scope so that you can interact with them. You may also be keen to peruse all of the steel
functions and modules available. Those can be found in `steel-docs.md`.


### Command API

There are two levels of the functionality exposed to plugins. The first is simply based around
chaining builtin commands, as if you're a lightning fast human typing commands very quickly. The other level
is a bit lower, and deals directly with the component API that helix uses to draw the text editor and various
popups, like the file picker or buffer selection.

To understand the first level, which is accessing typed commands and static commands, i.e. commands that you
typically type via `:`, or static commands, commands which are bound to keybindings, you can look at the modules:

* helix/commands.scm
* helix/static.scm

Every function here implicitly has access to a context, the helix context. This assumes that you're focused onto
some buffer, and any actions are assumed to be done within that context. For example, calling `vsplit` will
split the currently focused into a second, and move your focus to that window. Keeping track of that is important
to understand where your focus is.

In general, these functions do not return anything, given that they're purely for side effects. There are some functions
that do, and they should be documented as such. The API will need to be improved to return useful things where relevant.

### The UI

A good rule of thumb is to not block the UI. During the execution of a steel function, the helix context is exclusively
available to that executing function. As a result, you should not have long running functions there (note - if you end
up having an infinite loop of some kind, `ctrl-c` should break you out).

Luckily, there are a handful of ways we can accomplish more sophisticated plugins:

* Futures
* Threads

There are a handful of primitives that accept a future + a callback, where the callback will get executed once the future
is complete. The future will get scheduled on to the helix event loop, so the UI won't be blocked. (TODO: Document this more!)

Another way we can accomplish this is with native threads. Steel supports native threads, which means we can spawn a function
off on to another thread to run some code. Consider the following example which won't work:


```scheme
(spawn-native-thread (lambda () (sleep/ms 1000) (theme "focus_nova"))) ;; Note, this won't work!
```

This appears to spawn a thread, sleep for 1 second, and then change the theme. The issue here is that this thread does not
have control over the helix context. So what we'll have to do instead, is schedule a function to be run on the main thread:


```scheme
(require "helix/ext.scm")
(require-builtin steel/time)

(spawn-native-thread
  (lambda ()
    (hx.block-on-task
      (lambda ()
        (sleep/ms 1000)
        (theme "focus_nova")))))
```

`hx.block-on-task` will check if we're running on the main thread. If we are already, it doesn't do anything - but otherwise,
it enqueues a callback that schedules itself onto the main thread, and waits till it can acquire the helix context. The function
is then run, and the value returned back to this thread of control.


There is also `hx.with-context` which does a similar thing, except it does _not_ block the current thread.

### Components

Coming soon!
