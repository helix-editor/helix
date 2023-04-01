;; Includes

((symbol) @keyword.control.import
  (#match? @keyword.control.import "^(use|import|load)$"))

;; Keywords

((symbol) @keyword
  (#match? @keyword "^(do|doc)$"))

; Keywords construct a symbol

(keyword) @string.special.symbol

;; Operators

; TODO: classify
((symbol) @operator (#match? @operator "^(&|\\*|\\+|-|<|<=|=|>|>=)$"))

;; Defining

((list
  . (symbol) @label
  . (symbol) @function
  (symbol)? @parameter)
  (#match? @label "^(def|defop|defn)$"))

((cons
  . (symbol) @label
  . (symbol) @function
  (symbol)? @parameter)
  (#match? @label "^(def|defop|defn)$"))

((symbol) @label
  (#match? @label "^(def|defop|defn)$"))

;; Builtins

((symbol) @function.builtin
  (#match? @function.builtin "^(dump|mkfs|json|log|error|now|cons|wrap|unwrap|eval|make-scope|bind|meta|with-meta|null\\?|ignore\\?|boolean\\?|number\\?|string\\?|symbol\\?|scope\\?|sink\\?|source\\?|list\\?|pair\\?|applicative\\?|operative\\?|combiner\\?|path\\?|empty\\?|thunk\\?|\\+|\\*|quot|-|max|min|=|>|>=|<|<=|list->source|across|emit|next|reduce-kv|assoc|symbol->string|string->symbol|str|substring|trim|scope->list|string->fs-path|string->cmd-path|string->dir|subpath|path-name|path-stem|with-image|with-dir|with-args|with-cmd|with-stdin|with-env|with-insecure|with-label|with-port|with-tls|with-mount|thunk-cmd|thunk-args|resolve|start|addr|wait|read|cache-dir|binds\\?|recall-memo|store-memo|mask|list|list\\*|first|rest|length|second|third|map|map-pairs|foldr|foldl|concat|append|filter|conj|list->scope|merge|apply|id|always|vals|keys|memo|succeeds\\?|run|last|take|collect|take-all|insecure!|from|cd|wrap-cmd|mkfile|path-base|not)$"))

((symbol) @function.macro
  (#match? @function.macro "^(op|fn|current-scope|quote|let|provide|module|or|and|->|curryfn|assert|for|\\$|linux)$"))

;; Conditionals

((symbol) @keyword.control.conditional
  (#match? @keyword.control.conditional "^(if|case|cond|when)$"))

;; Repeats

((symbol) @keyword.control.repeat
  (#match? @keyword.control.repeat "^(each)$"))

;; Special forms

; (-> x y z) highlights first x as var, y z as function
(list
  .
  (symbol) @function.macro
  (#eq? @function.macro "->")
  .
  (symbol) @variable.parameter
  (symbol) @function)

; (-> 42 x y) highlights 42 as regular number
(list
  .
  (symbol) @function.macro
  (#eq? @function.macro "->")
  .
  (_)
  (symbol) @function)

;; Functions

(list
  . (symbol) @function)

;; Variables

(list (symbol) @variable)

(cons (symbol) @variable)

(scope (symbol) @variable)

(symbind (symbol) @variable)

;; Namespaces

(symbind
  (symbol) @namespace
  . (keyword))

;; Punctuation

[ "(" ")" ] @punctuation.bracket

[ "{" "}" ] @punctuation.bracket

[ "[" "]" ] @punctuation.bracket

((symbol) @punctuation.delimiter
  (#eq? @punctuation.delimiter "->"))

;; Literals

(string) @string

(escape_sequence) @constant.character.escape

(path) @string.special.path
(command) @string.special.path

(number) @constant.numeric.integer

(boolean) @constant.builtin.boolean

[
  (ignore)
  (null)
] @constant.builtin

[
  "^"
] @character.special

;; Comments

(comment) @comment.line @spell
