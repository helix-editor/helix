(kwd_lit) @string.special.symbol

(str_lit) @string

(long_str_lit) @string

(buf_lit) @string

(long_buf_lit) @string

(num_lit) @constant.numeric

[(bool_lit) (nil_lit)] @constant.builtin

(comment) @comment

((sym_lit) @variable
 (#match? @variable "^\\*.+\\*$"))

(short_fn_lit
 .
 (sym_lit) @function)

;; special forms
(par_tup_lit
 .
 (sym_lit) @function.macro
 (#match? @function.macro
  "^(break|def|do|fn|if|quasiquote|quote|set|splice|unquote|upscope|var|while)$"))

;; for macros
;;
;; (each name (all-bindings)
;;   (when-let [info (dyn (symbol name))]
;;     (when (info :macro)
;;       (print name))))
(par_tup_lit
 .
 (sym_lit) @function.macro
 (#match? @function.macro
  "^(%=|\\*=|\\+\\+|\\+=|\\-\\-|\\-=|\\->|\\->>|\\-\\?>|\\-\\?>>|/=|and|as\\->|as\\-macro|as\\?\\->|assert|case|catseq|chr|comment|compif|comptime|compwhen|cond|coro|def\\-|default|defdyn|defer|defmacro|defmacro\\-|defn|defn\\-|delay|doc|each|eachk|eachp|edefer|ev/do\\-thread|ev/gather|ev/spawn|ev/spawn\\-thread|ev/with\\-deadline|ffi/defbind|fiber\\-fn|for|forever|forv|generate|if\\-let|if\\-not|if\\-with|import|juxt|label|let|loop|match|or|prompt|protect|repeat|seq|short\\-fn|tabseq|toggle|tracev|try|unless|use|var\\-|varfn|when|when\\-let|when\\-with|with|with\\-dyns|with\\-syms|with\\-vars)$"))

;; builtin functions
;;
;; (each name (all-bindings)
;;   (when-let [info (dyn (symbol name))]
;;     (when (and (nil? (info :macro))
;;                (or (function? (info :value))
;;                    (cfunction? (info :value))))
;;       (print name))))
((sym_lit) @function.builtin
 (#match? @function.builtin
  "^(%|\\*|\\+|\\-|/|<|<=|=|>|>=|\\.break|\\.breakall|\\.bytecode|\\.clear|\\.clearall|\\.disasm|\\.fiber|\\.fn|\\.frame|\\.locals|\\.next|\\.nextc|\\.ppasm|\\.signal|\\.slot|\\.slots|\\.source|\\.stack|\\.step|abstract\\?|accumulate|accumulate2|all|all\\-bindings|all\\-dynamics|any\\?|apply|array|array/clear|array/concat|array/ensure|array/fill|array/insert|array/new|array/new\\-filled|array/peek|array/pop|array/push|array/remove|array/slice|array/trim|array/weak|array\\?|asm|bad\\-compile|bad\\-parse|band|blshift|bnot|boolean\\?|bor|brshift|brushift|buffer|buffer/bit|buffer/bit\\-clear|buffer/bit\\-set|buffer/bit\\-toggle|buffer/blit|buffer/clear|buffer/fill|buffer/format|buffer/from\\-bytes|buffer/new|buffer/new\\-filled|buffer/popn|buffer/push|buffer/push\\-at|buffer/push\\-byte|buffer/push\\-string|buffer/push\\-word|buffer/slice|buffer/trim|buffer\\?|bxor|bytes\\?|cancel|cfunction\\?|cli\\-main|cmp|comp|compare|compare<|compare<=|compare=|compare>|compare>=|compile|complement|count|curenv|debug|debug/arg\\-stack|debug/break|debug/fbreak|debug/lineage|debug/stack|debug/stacktrace|debug/step|debug/unbreak|debug/unfbreak|debugger|debugger\\-on\\-status|dec|deep\\-not=|deep=|defglobal|describe|dictionary\\?|disasm|distinct|div|doc\\*|doc\\-format|doc\\-of|dofile|drop|drop\\-until|drop\\-while|dyn|eflush|empty\\?|env\\-lookup|eprin|eprinf|eprint|eprintf|error|errorf|ev/acquire\\-lock|ev/acquire\\-rlock|ev/acquire\\-wlock|ev/all\\-tasks|ev/call|ev/cancel|ev/capacity|ev/chan|ev/chan\\-close|ev/chunk|ev/close|ev/count|ev/deadline|ev/full|ev/give|ev/give\\-supervisor|ev/go|ev/lock|ev/read|ev/release\\-lock|ev/release\\-rlock|ev/release\\-wlock|ev/rselect|ev/rwlock|ev/select|ev/sleep|ev/take|ev/thread|ev/thread\\-chan|ev/write|eval|eval\\-string|even\\?|every\\?|extreme|false\\?|ffi/align|ffi/call|ffi/calling\\-conventions|ffi/close|ffi/context|ffi/free|ffi/jitfn|ffi/lookup|ffi/malloc|ffi/native|ffi/pointer\\-buffer|ffi/pointer\\-cfunction|ffi/read|ffi/signature|ffi/size|ffi/struct|ffi/trampoline|ffi/write|fiber/can\\-resume\\?|fiber/current|fiber/getenv|fiber/last\\-value|fiber/maxstack|fiber/new|fiber/root|fiber/setenv|fiber/setmaxstack|fiber/status|fiber\\?|file/close|file/flush|file/lines|file/open|file/read|file/seek|file/tell|file/temp|file/write|filter|find|find\\-index|first|flatten|flatten\\-into|flush|flycheck|freeze|frequencies|from\\-pairs|function\\?|gccollect|gcinterval|gcsetinterval|gensym|get|get\\-in|getline|getproto|group\\-by|has\\-key\\?|has\\-value\\?|hash|idempotent\\?|identity|import\\*|in|inc|index\\-of|indexed\\?|int/s64|int/to\\-bytes|int/to\\-number|int/u64|int\\?|interleave|interpose|invert|juxt\\*|keep|keep\\-syntax|keep\\-syntax!|keys|keyword|keyword/slice|keyword\\?|kvs|last|length|lengthable\\?|load\\-image|macex|macex1|maclintf|make\\-env|make\\-image|map|mapcat|marshal|math/abs|math/acos|math/acosh|math/asin|math/asinh|math/atan|math/atan2|math/atanh|math/cbrt|math/ceil|math/cos|math/cosh|math/erf|math/erfc|math/exp|math/exp2|math/expm1|math/floor|math/gamma|math/gcd|math/hypot|math/lcm|math/log|math/log\\-gamma|math/log10|math/log1p|math/log2|math/next|math/pow|math/random|math/rng|math/rng\\-buffer|math/rng\\-int|math/rng\\-uniform|math/round|math/seedrandom|math/sin|math/sinh|math/sqrt|math/tan|math/tanh|math/trunc|max|max\\-of|mean|memcmp|merge|merge\\-into|merge\\-module|min|min\\-of|mod|module/add\\-paths|module/expand\\-path|module/find|module/value|nan\\?|nat\\?|native|neg\\?|net/accept|net/accept\\-loop|net/address|net/address\\-unpack|net/chunk|net/close|net/connect|net/flush|net/listen|net/localname|net/peername|net/read|net/recv\\-from|net/send\\-to|net/server|net/setsockopt|net/shutdown|net/write|next|nil\\?|not|not=|number\\?|odd\\?|one\\?|os/arch|os/cd|os/chmod|os/clock|os/compiler|os/cpu\\-count|os/cryptorand|os/cwd|os/date|os/dir|os/environ|os/execute|os/exit|os/getenv|os/isatty|os/link|os/lstat|os/mkdir|os/mktime|os/open|os/perm\\-int|os/perm\\-string|os/pipe|os/posix\\-exec|os/posix\\-fork|os/proc\\-close|os/proc\\-kill|os/proc\\-wait|os/readlink|os/realpath|os/rename|os/rm|os/rmdir|os/setenv|os/shell|os/sigaction|os/sleep|os/spawn|os/stat|os/strftime|os/symlink|os/time|os/touch|os/umask|os/which|pairs|parse|parse\\-all|parser/byte|parser/clone|parser/consume|parser/eof|parser/error|parser/flush|parser/has\\-more|parser/insert|parser/new|parser/produce|parser/state|parser/status|parser/where|partial|partition|partition\\-by|peg/compile|peg/find|peg/find\\-all|peg/match|peg/replace|peg/replace\\-all|pos\\?|postwalk|pp|prewalk|prin|prinf|print|printf|product|propagate|put|put\\-in|quit|range|reduce|reduce2|repl|require|resume|return|reverse|reverse!|run\\-context|sandbox|scan\\-number|setdyn|signal|slice|slurp|some|sort|sort\\-by|sorted|sorted\\-by|spit|string|string/ascii\\-lower|string/ascii\\-upper|string/bytes|string/check\\-set|string/find|string/find\\-all|string/format|string/from\\-bytes|string/has\\-prefix\\?|string/has\\-suffix\\?|string/join|string/repeat|string/replace|string/replace\\-all|string/reverse|string/slice|string/split|string/trim|string/triml|string/trimr|string\\?|struct|struct/getproto|struct/proto\\-flatten|struct/to\\-table|struct/with\\-proto|struct\\?|sum|symbol|symbol/slice|symbol\\?|table|table/clear|table/clone|table/getproto|table/new|table/proto\\-flatten|table/rawget|table/setproto|table/to\\-struct|table/weak|table/weak\\-keys|table/weak\\-values|table\\?|take|take\\-until|take\\-while|tarray/buffer|tarray/copy\\-bytes|tarray/length|tarray/new|tarray/properties|tarray/slice|tarray/swap\\-bytes|thread/close|thread/current|thread/exit|thread/new|thread/receive|thread/send|thaw|trace|true\\?|truthy\\?|tuple|tuple/brackets|tuple/setmap|tuple/slice|tuple/sourcemap|tuple/type|tuple\\?|type|unmarshal|untrace|update|update\\-in|values|varglobal|walk|warn\\-compile|xprin|xprinf|xprint|xprintf|yield|zero\\?|zipcoll)$"))

;; other calls
(par_tup_lit
 .
 (sym_lit) @function)

(sym_lit) @variable

["{" "@{" "}"
 "[" "@[" "]"
 "(" "@(" ")"] @punctuation.bracket

["~" "'" "|" ";" ","] @operator
