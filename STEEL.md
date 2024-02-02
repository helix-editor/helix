# Building

You will need a handful of things:

* A clone of this fork, on the branch `mwp-steel-integration`
* A clone of the steel git repo -> https://github.com/mattwparas/steel, on the branch `master` (default)

I also cannot promise that this will work on windows. I develop off of ubuntu and mac, so for now you can probably safely assume it will work on unix.

The `Cargo.toml` for helix points to a local development version of steel. Set this up so that it points to wherever you've cloned steel:

```
[workspace.dependencies]
# CHANGE 'path = ...' to point to the path to steel-core
steel-core = { path = "../../steel/crates/steel-core", version = "0.5.0", features = ["modules", "anyhow", "dylibs", "colors"] }
```

Since I'm actively developing steel alongside the helix integration in order to make things as smooth as possible, its not referencing a published version yet.

## Installing Steel

Follow the instructions here https://github.com/mattwparas/steel and https://github.com/mattwparas/steel/issues/71

Setting a `STEEL_HOME` env var, then running `cargo run -- cogs/install.scm` in the root of that repo will set up the steel core libraries so that helix can reference them.

## Installing helix

Once you're set up with steel, just run

`cargo install --path helix-term --locked`

To install the `hx` executable, with steel as the plugin language.


## Setting up configurations for helix

Note, this API is entirely subjet to change, and I promise absolutely 0 backwards compatibility while this is in development.

There are 2 important files you'll want:

* `~/.config/helix/helix.scm`
* `~/.config/helix/init.scm`

Note - these both live inside the same directory that helix sets up for runtime configurations.


### `helix.scm`

The `helix.scm` module will be loaded first before anything else, the runtime will `require` this module, and any functions exported will now be available
to be used as typed commands. For example:


```scheme
# helix.scm

(require-builtin helix/core/typable as helix.)
(require-builtin helix/core/static as helix.static.)
(require-builtin helix/core/keybindings as helix.keybindings.)

(provide shell git-add open-helix-scm open-init-scm reload-helix-scm)

;;@doc
;; Specialized shell implementation, where % is a wildcard for the current file
(define (shell cx . args)
  ;; Replace the % with the current file
  (define expanded (map (lambda (x) (if (equal? x "%") (current-path cx) x)) args))
  (helix.run-shell-command cx expanded helix.PromptEvent::Validate))

;;@doc
;; Adds the current file to git	
(define (git-add cx)
  (shell cx "git" "add" "%"))


;; Functions to assist with the above

(define (editor-get-doc-if-exists editor doc-id)
  (if (editor-doc-exists? editor doc-id) (editor->get-document editor doc-id) #f))

(define (current-path cx)
  (let* ([editor (cx-editor! cx)]
         [focus (editor-focus editor)]
         [focus-doc-id (editor->doc-id editor focus)]
         [document (editor-get-doc-if-exists editor focus-doc-id)])

    (if document (Document-path document) #f)))


;;@doc
;; Reload the helix.scm file
(define (reload-helix-scm cx)
  (helix.static.run-in-engine! cx
                               (string-append "(require \"" (helix.static.get-helix-scm-path) "\")")))

;;@doc
;; Open the helix.scm file
(define (open-helix-scm cx)
  (helix.open cx (list (helix.static.get-helix-scm-path)) helix.PromptEvent::Validate))

;;@doc
;; Opens the init.scm file
(define (open-init-scm cx)
  (helix.open cx (list (helix.static.get-init-scm-path)) helix.PromptEvent::Validate))


  
	
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
(require-builtin helix/core/static as helix.static.)
(require-builtin helix/core/typable as helix.)


(define rng (rand::thread-rng!))


;; Picking one from the possible themes
(define possible-themes '("ayu_mirage" "tokyonight_storm" "catppuccin_macchiato"))

(define (select-random lst)
  (let ([index (rand::rng->gen-range rng 0 (length lst))]) (list-ref lst index)))

(define (randomly-pick-theme options)
  ;; Randomly select the theme from the possible themes list
  (helix.theme *helix.cx* (list (select-random options)) helix.PromptEvent::Validate))

(randomly-pick-theme possible-themes)

```

### Libraries for helix

There are a handful of extra libraries in development for extending helix, and can be found here https://github.com/mattwparas/helix-config.

If you'd like to use them, create a directory called `cogs` in your `.config/helix` directory, and copy the files in there. In particular, `keymaps.scm` and `options.scm` are working well.

### options.scm

If you'd like to override configurations from your toml config:


```scheme
# init.scm

(require (only-in "cogs/options.scm" apply-options))

(define *config-map* '((file-picker.hidden false) (cursorline true) (soft-wrap.enable true)))
(apply-options *helix.cx* *config-map*)

```


### keymaps.scm

Applying custom keybindings for certain file extensions:


```scheme
# init.scm

(require "cogs/keymaps.scm")


(define scm-keybindings
  (hash 
        "insert"
        (hash "ret" ':scheme-indent)))

				
;; Grab whatever the existing keybinding map is
(define standard-keybindings (helix-current-keymap))

;; Overlay the scm keybindings on top of the standard keybindings. This does a little mutation here, so its a bit funky looking.
(merge-keybindings standard-keybindings scm-keybindings)

;; For .scm files, use this keybinding set insteead
(set-global-buffer-or-extension-keymap (hash "scm" standard-keybindings))
	
```

In insert mode, this overrides the `ret` keybinding to instead use a custom scheme indent function. Functions _must_ be available as typed commands, and are referred to
as symbols. So in this case, the `scheme-indent` function was exported by my `helix.scm` module.
