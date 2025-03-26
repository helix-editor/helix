; GENERATED VIA https://github.com/vito/tree-sitter-bass

;;; comments

(comment) @comment.line

;;; punctuation

["(" ")" "[" "]" "{" "}"] @punctuation.bracket

;;; constants

[(ignore) (null)] @constant.builtin

(bool) @constant.builtin.boolean

(int) @constant.numeric.integer

;;; strings

;; string literals

(string) @string
(string (string_escape) @constant.character.escape)

;; keywords (symbol literals)

(keyword) @string.special.symbol

;; paths

(dot) @string.special.path
(dotdot) @string.special.path
(command) @string.special.path
(subpath (symbol) @string.special.path)

; slashes in a path denote a combiner call
(subpath (slash) @function)


;;; generic highlighting for all forms

; first symbol in a list form is a combiner call
(list . (symbol) @function)

; highlight symbols as vars only when they're clearly vars
(cons (symbol) @variable)
(scope (symbol) @variable)
(path form: (symbol) @variable)
(symbind form: (symbol) @variable)


;;; specific highlighting for builtins & special forms

;; symbol classification based highlighting

(list . (symbol) @keyword.control.conditional (#match? @keyword.control.conditional "^(if|case|cond|when)$"))
(cons . (symbol) @keyword.control.conditional (#match? @keyword.control.conditional "^(if|case|cond|when)$"))

(list . (symbol) @keyword.control.repeat (#match? @keyword.control.repeat "^(each)$"))
(cons . (symbol) @keyword.control.repeat (#match? @keyword.control.repeat "^(each)$"))

(list . (symbol) @label (#match? @label "^(def|defop|defn)$"))
(cons . (symbol) @label (#match? @label "^(def|defop|defn)$"))

(list . (symbol) @function.builtin (#match? @function.builtin "^(dump|mkfs|json|log|error|now|cons|wrap|unwrap|eval|make-scope|bind|meta|with-meta|null\\?|ignore\\?|boolean\\?|number\\?|string\\?|symbol\\?|scope\\?|sink\\?|source\\?|list\\?|pair\\?|applicative\\?|operative\\?|combiner\\?|path\\?|empty\\?|thunk\\?|\\+|\\*|quot|-|max|min|=|>|>=|<|<=|list->source|across|emit|next|reduce-kv|assoc|symbol->string|string->symbol|str|substring|trim|scope->list|string->fs-path|string->cmd-path|string->dir|subpath|path-name|path-stem|with-image|with-dir|with-args|with-cmd|with-stdin|with-env|with-insecure|with-label|with-port|with-tls|with-mount|thunk-cmd|thunk-args|resolve|start|addr|wait|read|cache-dir|binds\\?|recall-memo|store-memo|mask|list|list\\*|first|rest|length|second|third|map|map-pairs|foldr|foldl|append|filter|conj|list->scope|merge|apply|id|always|vals|keys|memo|succeeds\\?|run|last|take|take-all|insecure!|from|cd|wrap-cmd|mkfile|path-base|not)$"))
(cons . (symbol) @function.builtin (#match? @function.builtin "^(dump|mkfs|json|log|error|now|cons|wrap|unwrap|eval|make-scope|bind|meta|with-meta|null\\?|ignore\\?|boolean\\?|number\\?|string\\?|symbol\\?|scope\\?|sink\\?|source\\?|list\\?|pair\\?|applicative\\?|operative\\?|combiner\\?|path\\?|empty\\?|thunk\\?|\\+|\\*|quot|-|max|min|=|>|>=|<|<=|list->source|across|emit|next|reduce-kv|assoc|symbol->string|string->symbol|str|substring|trim|scope->list|string->fs-path|string->cmd-path|string->dir|subpath|path-name|path-stem|with-image|with-dir|with-args|with-cmd|with-stdin|with-env|with-insecure|with-label|with-port|with-tls|with-mount|thunk-cmd|thunk-args|resolve|start|addr|wait|read|cache-dir|binds\\?|recall-memo|store-memo|mask|list|list\\*|first|rest|length|second|third|map|map-pairs|foldr|foldl|append|filter|conj|list->scope|merge|apply|id|always|vals|keys|memo|succeeds\\?|run|last|take|take-all|insecure!|from|cd|wrap-cmd|mkfile|path-base|not)$"))

(list . (symbol) @function.macro (#match? @function.macro "^(op|fn|current-scope|quote|let|provide|module|or|and|->|curryfn|for|\\$|linux)$"))
(cons . (symbol) @function.macro (#match? @function.macro "^(op|fn|current-scope|quote|let|provide|module|or|and|->|curryfn|for|\\$|linux)$"))

(list . (symbol) @keyword.builtin (#match? @keyword.builtin "^(do|doc)$"))
(cons . (symbol) @keyword.builtin (#match? @keyword.builtin "^(do|doc)$"))

(list . (symbol) @keyword.control.import (#match? @keyword.control.import "^(use|import|load)$"))
(cons . (symbol) @keyword.control.import (#match? @keyword.control.import "^(use|import|load)$"))


;; special cases

; [a & b] highlights & as operator rather than a regular symbol
(list (symbol) @operator (#match? @operator "&"))
(cons (symbol) @operator (#match? @operator "&"))

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
