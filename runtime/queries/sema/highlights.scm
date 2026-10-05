; Sema highlight queries for Helix
; Helix uses LAST-MATCH-WINS semantics, so generic fallbacks go FIRST
; and increasingly specific patterns follow.
; See: https://docs.helix-editor.com/themes.html for capture names.

; =====================================================================
; GENERIC FALLBACKS (least specific — put first so they lose to later)
; =====================================================================

; --- Fallback: any remaining symbol is a variable ---

(symbol) @variable

; --- Generic function call ---

(list
  .
  (symbol) @function)

; =====================================================================
; INCREASINGLY SPECIFIC PATTERNS (override the fallbacks above)
; =====================================================================

; --- Literals ---

(number) @constant.numeric
(boolean) @constant.builtin.boolean
(character) @constant.character
(string) @string
(regex) @string.regexp

; Reader extensions: deref (@expr) and shebang (#!...)
(deref "@" @operator)
(shebang) @comment

; --- Comments ---

(comment) @comment.line
(block_comment) @comment.block

; --- Punctuation ---

["(" ")" "[" "]" "{" "}"] @punctuation.bracket

; --- Dot as punctuation (variadic args, dotted pairs) ---

((symbol) @punctuation.delimiter
  (#eq? @punctuation.delimiter "."))

; --- Ellipsis ---

((symbol) @variable.builtin
  (#eq? @variable.builtin "..."))

; --- Operators ---

((symbol) @operator
  (#any-of? @operator
    "+" "-" "*" "/" "%" "=" ">" "<" ">=" "<="
    "eq?" "equal?" "eqv?"))

; --- Keyword literals :foo (map keys, keyword args) ---

(keyword) @string.special.symbol

; --- Keyword accessor in call position: (:name person) ---

(list
  .
  (keyword) @function.method)

; --- nil ---

((symbol) @constant.builtin
  (#eq? @constant.builtin "nil"))

; --- Quote operators ---

(quote "'" @operator)
(quasiquote "`" @operator)
(unquote "," @operator)
(unquote_splicing ",@" @operator)

; --- Define simple variable: (define name value) ---

(list
  .
  (symbol) @_f
  .
  (symbol) @variable
  (#any-of? @_f "define" "def"))

; --- set! target: (set! name value) ---

(list
  .
  (symbol) @_f
  .
  (symbol) @variable
  (#eq? @_f "set!"))

; --- Define function: (defun name ...) ---

(list
  .
  (symbol) @_f
  .
  (symbol) @function
  (#any-of? @_f "defun" "defn"))

; --- Define macro: (defmacro name ...) ---

(list
  .
  (symbol) @_f
  .
  (symbol) @function
  (#eq? @_f "defmacro"))

; --- Define agent: (defagent name ...) ---

(list
  .
  (symbol) @_f
  .
  (symbol) @function
  (#eq? @_f "defagent"))

; --- Define tool: (deftool name ...) ---

(list
  .
  (symbol) @_f
  .
  (symbol) @function
  (#eq? @_f "deftool"))

; --- Define workflow or policy: (defworkflow name ...) / (defpolicy name ...) ---

(list
  .
  (symbol) @_f
  .
  (symbol) @function
  (#any-of? @_f "defworkflow" "defpolicy"))

; --- Lambda / fn parameter lists ---

(list
  .
  (symbol) @_f
  .
  (list
    (symbol) @variable.parameter)
  (#any-of? @_f "lambda" "fn"))

; --- let-binding variables ---

(list
  .
  (symbol) @_f
  .
  (list
    (list
      (symbol) @variable.parameter))
  (#any-of? @_f "let" "let*" "letrec" "do"))

; --- Define with inline parameter list: (define (name args...) body) ---

(list
  .
  (symbol) @_f
  .
  (list
    .
    (symbol) @function
    (symbol) @variable.parameter)
  (#any-of? @_f "define" "def"))

; --- Canonical documented symbols ---
; Generated from crates/sema-docs/builtin_docs.generated.json.
; The later rules keep special forms and operators in their specific classes.

(list
  .
  (symbol) @function.builtin
  (#any-of? @function.builtin
    "&" "*" "*stderr*" "*stdin*" "*stdout*" "+" "-" "->" "->>" "/" "<" "<="
    "=" ">" ">=" "abs" "agent" "agent/max-turns" "agent/model" "agent/name" "agent/run" "agent/system" "agent/tools" "agent?"
    "and" "angle" "any" "any?" "append" "apply" "approval" "as->" "assert" "assert=" "assoc" "assoc-in"
    "assq" "assv" "async" "async/all" "async/await" "async/cancel" "async/cancelled?" "async/forced?" "async/map" "async/pending?" "async/pool-map" "async/promise?"
    "async/race" "async/race-owned" "async/rejected" "async/rejected?" "async/resolved" "async/resolved?" "async/run" "async/sleep" "async/spawn" "async/spawn-all" "async/timeout" "async/with-timeout"
    "await" "base64/decode" "base64/decode-bytes" "base64/encode" "base64/encode-bytes" "begin" "bit/and" "bit/not" "bit/or" "bit/shift-left" "bit/shift-right" "bit/xor"
    "bool?" "boolean?" "bytes/->string" "bytes/find" "bytes/length" "bytes/parse-int10" "bytes/ref" "bytes/slice" "bytevector" "bytevector->list" "bytevector-append" "bytevector-copy"
    "bytevector-length" "bytevector-u8-ref" "bytevector-u8-set!" "bytevector/append" "bytevector/copy" "bytevector/from-list" "bytevector/length" "bytevector/make" "bytevector/new" "bytevector/ref" "bytevector/set!" "bytevector/to-list"
    "bytevector/u8-ref" "bytevector/u8-set!" "bytevector?" "caaar" "caadr" "caar" "cadar" "cadr" "call-with-values" "car" "case" "cdaar"
    "cdadr" "cdar" "cddar" "cdddr" "cddr" "cdr" "ceil" "ceiling" "channel/close" "channel/closed?" "channel/count" "channel/empty?"
    "channel/full?" "channel/new" "channel/recv" "channel/send" "channel/try-recv" "channel?" "char-ci<=?" "char-ci<?" "char-ci=?" "char-ci>=?" "char-ci>?" "char-lower-case?"
    "char/alphabetic?" "char/downcase" "char/numeric?" "char/to-integer" "char/to-string" "char/upcase" "char/upper-case?" "char/whitespace?" "char<=?" "char<?" "char=?" "char>=?"
    "char>?" "char?" "checkpoint" "complex?" "cond" "cons" "contains?" "context/all" "context/clear" "context/get" "context/get-hidden" "context/has-hidden?"
    "context/has?" "context/merge" "context/pop" "context/pull" "context/push" "context/remove" "context/set" "context/set-hidden" "context/stack" "context/with" "conversation/add-message" "conversation/cost"
    "conversation/filter" "conversation/find" "conversation/fork" "conversation/insert" "conversation/last-reply" "conversation/length" "conversation/map" "conversation/map-role" "conversation/messages" "conversation/model" "conversation/models-used" "conversation/new"
    "conversation/remove" "conversation/replace" "conversation/say" "conversation/say-as" "conversation/search" "conversation/set-system" "conversation/stats" "conversation/system" "conversation/token-count" "conversation/turns" "conversation?" "cos"
    "count" "csv/encode" "csv/parse" "csv/parse-maps" "db/close" "db/exec" "db/exec-batch" "db/last-insert-id" "db/open" "db/open-memory" "db/query" "db/query-one"
    "db/tables" "deep-merge" "def" "defagent" "define" "define-record-type" "define-syntax" "define-values" "defmacro" "defmethod" "defmulti" "defn"
    "defpolicy" "deftool" "defun" "defworkflow" "delay" "denominator" "diff/apply" "diff/hunks" "diff/parse" "diff/stat" "diff/unified" "display"
    "dissoc" "do" "document/chunk" "document/create" "document/metadata" "document/text" "dotimes" "drop" "drop-while" "e" "embedding/->list" "embedding/length"
    "embedding/list->embedding" "embedding/ref" "empty?" "enumerate" "env" "eq?" "equal?" "error" "eval" "even?" "event/select" "every"
    "every?" "exact" "exact->inexact" "exact-integer-sqrt" "exact-integer?" "exact?" "exit" "export" "expt" "f64-array" "f64-array/dot" "f64-array/fold"
    "f64-array/from-list" "f64-array/length" "f64-array/make" "f64-array/map" "f64-array/range" "f64-array/ref" "f64-array/set!" "f64-array/sum" "f64-array?" "file/append" "file/copy" "file/delete"
    "file/exists?" "file/fold-lines" "file/fold-lines-bytes" "file/for-each-line" "file/glob" "file/info" "file/is-directory?" "file/is-file?" "file/is-symlink?" "file/list" "file/mkdir" "file/read"
    "file/read-bytes" "file/read-lines" "file/rename" "file/write" "file/write-bytes" "file/write-lines" "filter" "first" "flat-map" "flatten" "flatten-deep" "float"
    "float?" "floor" "fn" "fn?" "fold" "foldl" "foldr" "for" "for-each" "for-range" "force" "format"
    "format/form" "frequencies" "fs/unwatch" "fs/watch" "fs/watch-events" "gc/collect" "gc/stats" "gcd" "gensym" "get" "get-in" "git/changed-files"
    "git/current-branch" "git/diff" "git/diff-files" "git/ignore-matches?" "git/recent-files" "git/root" "git/status" "guard" "gzip/compress" "gzip/decompress" "hash-map" "hash-map?"
    "hash-ref" "hash/digest" "hash/hmac-sha256" "hash/md5" "hash/sha256" "hashmap/assoc" "hashmap/contains?" "hashmap/get" "hashmap/keys" "hashmap/new" "hashmap/to-map" "html/parse"
    "html/select" "html/select-text" "html/text" "http/created" "http/delete" "http/error" "http/file" "http/get" "http/html" "http/no-content" "http/not-found" "http/ok"
    "http/post" "http/put" "http/query" "http/redirect" "http/request" "http/router" "http/serve" "http/stream" "http/text" "http/websocket" "i64-array" "i64-array/from-list"
    "i64-array/make" "i64-array/range" "if" "if-let" "imag-part" "import" "inexact" "inexact->exact" "inexact?" "int" "integer/to-char" "integer?"
    "interpose" "io/eof?" "io/flush" "io/print-error" "io/println-error" "io/read-key" "io/read-key-timeout" "io/read-line" "io/read-many" "io/read-stdin" "io/tty-raw!" "io/tty-restore!"
    "io/with-raw-mode" "iota" "json/decode" "json/encode" "json/encode-pretty" "keys" "keyword/to-string" "keyword?" "kv/close" "kv/delete" "kv/get" "kv/keys"
    "kv/open" "kv/set" "lambda" "last" "lcm" "length" "let" "let*" "let*-values" "let-values" "letrec" "list"
    "list->bytevector" "list->string" "list->vector" "list/avg" "list/chunk" "list/contains?" "list/cross-join" "list/dedupe" "list/diff" "list/drop-last" "list/drop-while" "list/duplicates"
    "list/find" "list/group-by" "list/index-of" "list/interleave" "list/intersect" "list/join" "list/key-by" "list/max" "list/median" "list/min" "list/mode" "list/nth-or"
    "list/pad" "list/page" "list/pick" "list/pluck" "list/reject" "list/repeat" "list/shuffle" "list/sliding" "list/sole" "list/split-at" "list/sum" "list/take-last"
    "list/take-while" "list/times" "list/to-bytevector" "list/unique" "list?" "llm/auto-configure" "llm/batch" "llm/budget-remaining" "llm/cache-clear" "llm/cache-key" "llm/cache-stats" "llm/cassette-eject"
    "llm/cassette-load" "llm/cassette-save" "llm/chat" "llm/classify" "llm/clear-budget" "llm/compare" "llm/complete" "llm/configure" "llm/configure-embeddings" "llm/current-provider" "llm/default-provider" "llm/define-provider"
    "llm/embed" "llm/extract" "llm/extract-from-image" "llm/last-usage" "llm/list-providers" "llm/pmap" "llm/pricing-status" "llm/providers" "llm/rerank" "llm/reset-usage" "llm/send" "llm/session-usage"
    "llm/set-budget" "llm/set-default" "llm/set-pricing" "llm/similarity" "llm/stream" "llm/summarize" "llm/token-count" "llm/token-estimate" "llm/with-budget" "llm/with-cache" "llm/with-cassette" "llm/with-fallback"
    "llm/with-rate-limit" "load" "log" "log/debug" "log/error" "log/info" "log/warn" "macroexpand" "magnitude" "make-bytevector" "make-list" "make-parameter"
    "make-polar" "make-rectangular" "make-string" "map" "map-indexed" "map/assoc-in" "map/deep-merge" "map/entries" "map/except" "map/filter" "map/from-entries" "map/get-in"
    "map/map-keys" "map/map-vals" "map/new" "map/select-keys" "map/sort-keys" "map/update" "map/update-in" "map/zip" "map?" "mapcar" "markdown/frontmatter" "markdown/headings"
    "markdown/to-html" "match" "match*" "math/acos" "math/asin" "math/atan" "math/atan2" "math/clamp" "math/cosh" "math/degrees->radians" "math/exp" "math/format-fixed"
    "math/gcd" "math/infinite?" "math/infinity" "math/lcm" "math/lerp" "math/log10" "math/log2" "math/map-range" "math/nan" "math/nan?" "math/pow" "math/quotient"
    "math/radians->degrees" "math/random" "math/random-int" "math/remainder" "math/round-to" "math/sign" "math/sinh" "math/tan" "math/tanh" "max" "mcp/call" "mcp/close"
    "mcp/connect" "mcp/tools" "mcp/tools->sema" "member" "memory/append" "memory/messages" "memory/open" "merge" "message" "message/content" "message/role" "message/with-image"
    "message?" "min" "mod" "module" "modulo" "mutable-array/->vector" "mutable-array/get" "mutable-array/length" "mutable-array/new" "mutable-array/push!" "mutable-array/set!" "mutable-cell/get"
    "mutable-cell/new" "mutable-cell/set!" "negative?" "newline" "nil?" "not" "nth" "null?" "number->string" "number/to-string" "number?" "numerator"
    "odd?" "or" "otel/configure" "otel/event" "otel/llm-span" "otel/llm-usage" "otel/retrieval-span" "otel/set-attribute" "otel/set-attributes" "otel/set-status" "otel/span" "otel/tool-span"
    "otel/with-session" "pair?" "parallel" "parallel-settled" "parameterize" "partition" "patch/apply-file" "path/absolute" "path/absolute?" "path/canonicalize" "path/dir" "path/extension"
    "path/filename" "path/join" "path/relative-to" "path/stem" "path/within?" "pdf/extract-text" "pdf/extract-text-pages" "pdf/metadata" "pdf/page-count" "phase" "pi" "pii/detect"
    "pio/assemble" "pio/delay" "pio/in" "pio/irq" "pio/jmp" "pio/mov" "pio/nop" "pio/out" "pio/pull" "pio/push" "pio/set" "pio/side"
    "pio/wait" "pipeline" "pipeline-settled" "policy/without" "positive?" "pow" "pprint" "print" "print-error" "println" "println-error" "proc/close"
    "proc/close-stdin" "proc/exit-code" "proc/kill" "proc/read-stderr" "proc/read-stdout" "proc/run" "proc/running?" "proc/spawn" "proc/wait" "proc/write-stdin" "procedure?" "progn"
    "promise-forced?" "promise?" "prompt" "prompt/append" "prompt/concat" "prompt/diff" "prompt/difference" "prompt/fill" "prompt/intersection" "prompt/messages" "prompt/render" "prompt/set-system"
    "prompt/slots" "prompt/template" "prompt/union" "prompt?" "pty/close" "pty/exit-code" "pty/kill" "pty/read" "pty/resize" "pty/running?" "pty/spawn" "pty/wait"
    "pty/write" "quasiquote" "quote" "quotient" "raise" "range" "rational?" "rationalize" "read" "read-line" "read-many" "read-stdin"
    "read/all" "read/string" "real-part" "real?" "record?" "redact/spans" "reduce" "regex/find-all" "regex/match" "regex/match?" "regex/replace" "regex/replace-all"
    "regex/split" "remainder" "rest" "retry" "reverse" "round" "route/from-tools" "route/prefix" "secret/detect" "secret/redact" "sema/check-file" "sema/check-string"
    "serial/close" "serial/list" "serial/open" "serial/read-line" "serial/send" "serial/write" "set!" "settled-partition" "settled/err?" "settled/ok?" "shell" "shell/quote"
    "sin" "sleep" "some->" "some?" "sort" "sort-by" "spy" "sqrt" "step" "str" "stream/available?" "stream/byte-buffer"
    "stream/close" "stream/copy" "stream/flush" "stream/from-bytes" "stream/from-string" "stream/open-input" "stream/open-output" "stream/read" "stream/read-all" "stream/read-byte" "stream/read-line" "stream/readable?"
    "stream/to-bytes" "stream/to-string" "stream/type" "stream/write" "stream/write-byte" "stream/write-string" "stream?" "string->float" "string->number" "string-ci=?" "string/after" "string/after-last"
    "string/append" "string/before" "string/before-last" "string/between" "string/byte-length" "string/camel-case" "string/capitalize" "string/chars" "string/chop-end" "string/chop-start" "string/codepoints" "string/contains?"
    "string/empty?" "string/ends-with?" "string/ensure-end" "string/ensure-start" "string/foldcase" "string/from-codepoints" "string/headline" "string/index-of" "string/intern" "string/join" "string/kebab-case" "string/last-index-of"
    "string/length" "string/lines" "string/lower" "string/map" "string/normalize" "string/number?" "string/pad-left" "string/pad-right" "string/pascal-case" "string/ref" "string/remove" "string/repeat"
    "string/replace" "string/replace-first" "string/replace-last" "string/reverse" "string/slice" "string/snake-case" "string/split" "string/starts-with?" "string/take" "string/title-case" "string/to-char" "string/to-keyword"
    "string/to-list" "string/to-number" "string/to-symbol" "string/to-utf8" "string/trim" "string/trim-left" "string/trim-right" "string/truncate-width" "string/unwrap" "string/upper" "string/width" "string/word-wrap"
    "string/words" "string/wrap" "string?" "symbol/to-string" "symbol?" "sys/arch" "sys/args" "sys/check-signals" "sys/config-dir" "sys/cwd" "sys/elapsed" "sys/env-all"
    "sys/home-dir" "sys/hostname" "sys/interactive?" "sys/interner-stats" "sys/on-signal" "sys/os" "sys/pid" "sys/platform" "sys/sema-home" "sys/set-env" "sys/temp-dir" "sys/term-size"
    "sys/tty" "sys/user" "sys/which" "take" "take-while" "tap" "tar/create" "tar/extract" "term/bell" "term/black" "term/blue" "term/bold"
    "term/clear" "term/clear-below" "term/clear-line" "term/cursor-home" "term/cursor-position" "term/cyan" "term/dim" "term/disable-bracketed-paste" "term/disable-focus-events" "term/disable-kitty-keys!" "term/disable-mouse" "term/enable-bracketed-paste"
    "term/enable-focus-events" "term/enable-kitty-keys!" "term/enable-mouse" "term/enter-alt-screen" "term/flush" "term/gray" "term/green" "term/hide-cursor" "term/inverse" "term/italic" "term/leave-alt-screen" "term/magenta"
    "term/move-to" "term/query-cursor-position" "term/query-kitty-keys" "term/query-primary-da" "term/query-secondary-da" "term/red" "term/restore-cursor" "term/rgb" "term/save-cursor" "term/set-title" "term/show-cursor" "term/spinner-start"
    "term/spinner-stop" "term/spinner-update" "term/strikethrough" "term/strip" "term/style" "term/supports-kitty-keys?" "term/underline" "term/white" "term/with-alt-screen" "term/with-bracketed-paste" "term/with-focus-events" "term/with-kitty-keys"
    "term/with-mouse" "term/write-at" "term/yellow" "text/chunk" "text/chunk-by-separator" "text/clean-whitespace" "text/excerpt" "text/normalize-newlines" "text/split-sentences" "text/strip-html" "text/trim-indent" "text/truncate"
    "text/word-count" "throw" "time" "time-ms" "time/add" "time/date-parts" "time/diff" "time/format" "time/ms" "time/now" "time/parse" "time/tick"
    "toml/decode" "toml/encode" "tool/description" "tool/invoke" "tool/name" "tool/parameters" "tool/policy-subjects" "tool?" "tools->routes" "truncate" "try" "type"
    "type-of" "unless" "update-in" "utf8/to-string" "uuid/v4" "vals" "values" "vector" "vector->list" "vector-store/add" "vector-store/count" "vector-store/create"
    "vector-store/delete" "vector-store/open" "vector-store/save" "vector-store/search" "vector/cosine-similarity" "vector/distance" "vector/dot-product" "vector/normalize" "vector?" "web/user-agent" "web/user-agent-data" "when"
    "when-let" "while" "with-open" "with-retry" "with-session" "with-span" "with-stream" "workflow/approval" "workflow/check" "workflow/checkpoint" "workflow/mcp-handle" "workflow/phase"
    "workflow/policy-without" "workflow/run" "workflow/run-form" "workflow/step" "workflow/tool-call" "workflow/tool-result" "ws/close" "ws/connect" "ws/connected?" "ws/listen" "ws/ping" "ws/recv"
    "ws/recv-timeout" "ws/send" "zero?" "zip" "zip/create" "zip/extract" "zip/list"))

; --- Threading macros ---

(list
  .
  (symbol) @keyword.operator
  (#any-of? @keyword.operator
    "->" "->>" "as->"))

; --- Module / import ---

(list
  .
  (symbol) @keyword.import
  (#any-of? @keyword.import
    "import" "module" "export" "load"))

; --- Exception handling ---

(list
  .
  (symbol) @keyword.exception
  (#any-of? @keyword.exception
    "try" "catch" "throw"))

; --- Conditionals ---

(list
  .
  (symbol) @keyword.conditional
  (#any-of? @keyword.conditional
    "if" "cond" "case" "when" "unless" "else"))

; --- LLM-specific special forms ---

(list
  .
  (symbol) @keyword.function
  (#any-of? @keyword.function
    "defagent" "deftool"))

; SPECIAL FORMS (most specific; last wins)
; =====================================================================

(list . (symbol) @keyword
  (#any-of? @keyword
    "define" "def" "defun" "defn" "lambda" "fn" "set!"
    "let" "let*" "letrec" "begin" "progn" "do" "while"
    "let-values" "let*-values" "define-values" "define-syntax"
    "match" "match*" "defmulti" "defmethod" "async" "await"
    "and" "or"
    "quote" "quasiquote" "unquote" "unquote-splicing"
    "define-record-type" "defmacro" "defagent" "deftool"
    "defworkflow" "defpolicy"
    "try" "catch" "throw"
    "import" "module" "export" "load"
    "delay" "force" "eval" "macroexpand"
    "guard" "when-let" "if-let" "with-stream" "with-open"
    "with-span" "with-session" "with-retry"
    "io/with-raw-mode" "term/with-alt-screen" "term/with-mouse"
    "llm/with-budget" "policy/without"
    "prompt" "message" "else"))

; Bound runtime aliases omitted from the documentation index.
(list . (symbol) @function.builtin
  (#any-of? @function.builtin
    "caddr"
    "char->integer"
    "char->string"
    "char-alphabetic?"
    "char-downcase"
    "char-numeric?"
    "char-upcase"
    "char-upper-case?"
    "char-whitespace?"
    "i64-array/fold"
    "i64-array/length"
    "i64-array/map"
    "i64-array/ref"
    "i64-array/set!"
    "i64-array/sum"
    "i64-array?"
    "integer->char"
    "keyword->string"
    "path/basename"
    "path/dirname"
    "path/ext"
    "stream/writable?"
    "string->char"
    "string->keyword"
    "string->list"
    "string->symbol"
    "string->utf8"
    "string-append"
    "string-length"
    "string-ref"
    "substring"
    "symbol->string"
    "time/now-ms"
    "utf8->string"))
