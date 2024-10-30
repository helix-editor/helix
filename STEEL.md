# Building

You will need:

* A clone of this fork, on the branch `steel-event-system`

`steel` is included as a git submodule for ease of building.

## Installing helix

Just run

`cargo xtask steel`

To install the `hx` executable, with steel as a plugin language. This also includes:

The `steel` executable, the steel language server, the steel dylib installer, and the steel standard library.


## Setting up configurations for helix

Note, this API is entirely subjet to change, and I promise absolutely 0 backwards compatibility while this is in development.

There are 2 important files you'll want, which should be auto generated during the installation process:

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

(define rng (rand::thread-rng!))

;; Picking one from the possible themes
(define possible-themes '("ayu_mirage" "tokyonight_storm" "catppuccin_macchiato"))

(define (select-random lst)
  (let ([index (rand::rng->gen-range rng 0 (length lst))]) (list-ref lst index)))

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
