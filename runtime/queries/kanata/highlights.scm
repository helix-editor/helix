(line_comment) @comment.line
(block_comment) @comment.block

(string) @string
(raw_string) @string

(number) @number @constant.numeric

(variable_reference) @variable
(alias_reference) @label

[
  "("
  ")"
] @punctuation.bracket

((identifier) @string.special.symbol
  (#match? @string.special.symbol "^(RS|RC|RA|RM|AG|[SCAMO])-"))

((identifier) @constant.builtin
  (#any-of? @constant.builtin
    "_" "__" "___" "‗" "≝"
    "XX" "✗" "∅" "•"))

((identifier) @constant.builtin.boolean
  (#any-of? @constant.builtin.boolean "yes" "no" "true" "false"))

((identifier) @function.builtin
  (#any-of? @function.builtin
    "lrld" "lrld-next" "lrnx" "lrld-prev" "lrpv"
    "rpt" "repeat" "rpt-key" "rpt-any"
    "sldr" "scnl" "use-defsrc" "reverse-release-order"))

;; List-action builtins  (head of a parenthesised form)
(list
  head: (identifier) @function.builtin
  (#any-of? @function.builtin
    "layer-switch" "layer-toggle" "layer-while-held"
    "tap-hold" "tap-hold-press" "tap-hold-release"
    "tap-hold-press-timeout" "tap-hold-release-timeout"
    "tap-hold-release-keys" "tap-hold-except-keys"
    "tap-hold-tap-keys" "tap-hold-keys" "tap-hold-order"
    "tap-hold-opposite-hand" "tap-hold-opposite-hand-release"
    "macro" "macro-repeat" "macro-release-cancel"
    "macro-repeat-release-cancel" "macro-cancel-on-press"
    "macro-repeat-cancel-on-press"
    "macro-release-cancel-and-cancel-on-press"
    "macro-repeat-release-cancel-and-cancel-on-press"
    "dynamic-macro-record" "dynamic-macro-play"
    "dynamic-macro-record-stop-truncate"
    "one-shot" "one-shot-press" "one-shot-release"
    "one-shot-press-pcancel" "one-shot-release-pcancel"
    "one-shot-pause-processing"
    "caps-word" "caps-word-toggle" "caps-word-custom"
    "caps-word-custom-toggle"
    "multi" "unicode" "chord" "tap-dance" "tap-dance-eager"
    "switch" "sequence" "sequence-noerase" "fork" "unmod" "unshift"
    "release-key" "release-layer"
    "on-press-fakekey" "on-release-fakekey"
    "on-press-delay" "on-release-delay"
    "on-press-fakekey-delay" "on-release-fakekey-delay"
    "on-idle-fakekey" "on-press" "on-release" "on-idle"
    "on-physical-idle" "hold-for-duration"
    "mwheel-up" "mwheel-down" "mwheel-left" "mwheel-right"
    "mwheel-accel-up" "mwheel-accel-down" "mwheel-accel-left"
    "mwheel-accel-right"
    "movemouse-up" "movemouse-down" "movemouse-left"
    "movemouse-right" "movemouse-accel-up" "movemouse-accel-down"
    "movemouse-accel-left" "movemouse-accel-right"
    "movemouse-speed" "setmouse"
    "arbitrary-code" "cmd" "cmd-log" "cmd-output-keys" "push-msg"
    "clipboard-set" "clipboard-cmd-set" "clipboard-save"
    "clipboard-restore" "clipboard-save-set"
    "clipboard-save-cmd-set" "clipboard-save-swap"
    "live-reload-num" "live-reload-file"))

(list
  head: (identifier) @function.macro
  (#any-of? @function.macro "template-expand" "t!" "concat"))

(list
  head: (identifier) @keyword.control.conditional
  (#any-of? @keyword.control.conditional
    "if-equal" "if-not-equal" "if-in-list" "if-not-in-list"))

(list
  head: (identifier) @keyword
  (#any-of? @keyword
    "defcfg" "defsrc" "deflayer" "deflayer-mapped" "deflayermap"
    "defalias" "defaliasenvcond" "defvar" "deftemplate"
    "deffakekeys" "defvirtualkeys" "defchords" "defchordsv2"
    "defchordsv2-experimental" "defzippy" "defzippy-experimental"
    "defseq" "defhands" "definputdevices"
    "defoverrides" "defoverridesv2"
    "deflocalkeys-macos" "deflocalkeys-linux" "deflocalkeys-win"
    "deflocalkeys-winiov2" "deflocalkeys-wintercept"
    "platform" "environment"))

(list
  head: (identifier) @keyword.control.import
  (#eq? @keyword.control.import "include"))

(list
  head: (identifier) @_include
  (#eq? @_include "include")
  body: [(string) (identifier)] @string.special.path)

[
  "defcfg"
  "defalias"
  "defvar"
  "defsrc"
  "deflayer"
  "deflayermap"
  "include"
  "deflocalkeys-win"
  "deflocalkeys-winiov2"
  "deflocalkeys-wintercept"
  "deflocalkeys-linux"
  "deflocalkeys-macos"
] @keyword

;; Config keys and typed definition-site names
(defcfg key: (identifier) @property)
(defalias name: (identifier) @variable.parameter)
(defvar name: (identifier) @variable)
(defsrc keys: (identifier) @type)

;; Layer names are modules/namespaces
(deflayer name: (identifier) @module)
(deflayermap name: (identifier) @module)
(deflayermap input: (identifier) @type)

;; Include path
(include file: [(string) (identifier)] @string.special.path)

;; Layer names inside generic-list deflayer / deflayer-mapped / defchords / deftemplate
(list
  head: (identifier) @_def
  (#any-of? @_def "deflayer" "deflayer-mapped" "defchords" "deftemplate")
  body: (identifier) @module)

(list
  head: (identifier) @_def
  (#eq? @_def "deflayermap")
  body: (list (identifier) @module))

;; Catch-all fallback: any bare identifier that isn't captured above
(identifier) @variable.other
