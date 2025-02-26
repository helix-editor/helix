(dis_expr) @comment

(kwd_lit) @string.special.symbol

(str_lit) @string

(num_lit) @constant.numeric

[(bool_lit) (nil_lit)] @constant.builtin

(comment) @comment

;; other symbols
(sym_lit) @variable

;; other calls
(list_lit
 .
 (sym_lit) @function)

;; metadata experiment
(meta_lit
 marker: "^" @punctuation)

;; dynamic variables
((sym_lit) @variable
 (#match? @variable "^\\*.+\\*$"))

;; parameter-related
((sym_lit) @variable.parameter
 (#match? @variable.parameter "^&.*$"))

;; gensym
((sym_lit) @variable
 (#match? @variable "^.*#$"))

;; def-like things
(list_lit
 .
 (sym_lit) @function.macro
 .
 (sym_lit) @function
 (#match? @function.macro "^(declare|def|definline|definterface|defmacro|defmethod|defmulti|defn|defn-|defonce|defprotocol|defstruct|deftype|ns)$"))

;; other macros
(list_lit
 .
 (sym_lit) @function.macro
 (#match? @function.macro "^(\\.|\\.\\.|\\->|\\->>|amap|and|areduce|as\\->|assert|binding|bound\\-fn|case|catch|comment|cond|cond\\->|cond\\->>|condp|delay|do|doseq|dosync|dotimes|doto|extend-protocol|extend-type|finally|fn|fn\\*|for|future|gen-class|gen-interface|if|if\\-let|if\\-not|if\\-some|import|io!|lazy\\-cat|lazy\\-seq|let|letfn|locking|loop|memfn|monitor\\-enter|monitor\\-exit|or|proxy|proxy-super|pvalues|quote|recur|refer\\-clojure|reify|set!|some\\->|some\\->>|sync|throw|time|try|unquote|unquote\\-splicing|var|vswap!|when|when\\-first|when\\-let|when\\-not|when\\-some|while|with\\-bindings|with\\-in\\-str|with\\-loading\\-context|with\\-local\\-vars|with\\-open|with\\-out\\-str|with\\-precision|with\\-redefs)$"))

(anon_fn_lit
 .
 (sym_lit) @function.macro
 (#match? @function.macro "^(\\.|\\.\\.|\\->|\\->>|amap|and|areduce|as\\->|assert|binding|bound\\-fn|case|catch|comment|cond|cond\\->|cond\\->>|condp|delay|do|doseq|dosync|dotimes|doto|extend-protocol|extend-type|finally|fn|fn\\*|for|future|gen-class|gen-interface|if|if\\-let|if\\-not|if\\-some|import|io!|lazy\\-cat|lazy\\-seq|let|letfn|locking|loop|memfn|monitor\\-enter|monitor\\-exit|or|proxy|proxy-super|pvalues|quote|recur|refer\\-clojure|reify|set!|some\\->|some\\->>|sync|throw|time|try|unquote|unquote\\-splicing|var|vswap!|when|when\\-first|when\\-let|when\\-not|when\\-some|while|with\\-bindings|with\\-in\\-str|with\\-loading\\-context|with\\-local\\-vars|with\\-open|with\\-out\\-str|with\\-precision|with\\-redefs)$"))

;; clojure.core=> (cp/pprint (sort (keep (fn [[s v]] (when-not (:macro (meta v)) s)) (ns-publics *ns*))))
;; ...and then some manual filtering...
(list_lit
 .
 (sym_lit) @function.builtin
 (#match? @function.builtin "^(\\*|\\*'|\\+|\\+'|\\-|\\-'|\\->ArrayChunk|\\->Eduction|\\->Vec|\\->VecNode|\\->VecSeq|\\-cache\\-protocol\\-fn|\\-reset\\-methods|/|<|<=|=|==|>|>=|PrintWriter\\-on|StackTraceElement\\->vec|Throwable\\->map|accessor|aclone|add\\-classpath|add\\-tap|add\\-watch|agent|agent\\-error|agent\\-errors|aget|alength|alias|all\\-ns|alter|alter\\-meta!|alter\\-var\\-root|ancestors|any\\?|apply|array\\-map|aset|aset\\-boolean|aset\\-byte|aset\\-char|aset\\-double|aset\\-float|aset\\-int|aset\\-long|aset\\-short|assoc|assoc!|assoc\\-in|associative\\?|atom|await|await\\-for|await1|bases|bean|bigdec|bigint|biginteger|bit\\-and|bit\\-and\\-not|bit\\-clear|bit\\-flip|bit\\-not|bit\\-or|bit\\-set|bit\\-shift\\-left|bit\\-shift\\-right|bit\\-test|bit\\-xor|boolean|boolean\\-array|boolean\\?|booleans|bound\\-fn\\*|bound\\?|bounded\\-count|butlast|byte|byte\\-array|bytes|bytes\\?|cast|cat|char|char\\-array|char\\-escape\\-string|char\\-name\\-string|char\\?|chars|chunk|chunk\\-append|chunk\\-buffer|chunk\\-cons|chunk\\-first|chunk\\-next|chunk\\-rest|chunked\\-seq\\?|class|class\\?|clear\\-agent\\-errors|clojure\\-version|coll\\?|commute|comp|comparator|compare|compare\\-and\\-set!|compile|complement|completing|concat|conj|conj!|cons|constantly|construct\\-proxy|contains\\?|count|counted\\?|create\\-ns|create\\-struct|cycle|dec|dec'|decimal\\?|dedupe|default\\-data\\-readers|delay\\?|deliver|denominator|deref|derive|descendants|destructure|disj|disj!|dissoc|dissoc!|distinct|distinct\\?|doall|dorun|double|double\\-array|double\\?|doubles|drop|drop\\-last|drop\\-while|eduction|empty|empty\\?|ensure|ensure\\-reduced|enumeration\\-seq|error\\-handler|error\\-mode|eval|even\\?|every\\-pred|every\\?|ex\\-cause|ex\\-data|ex\\-info|ex\\-message|extend|extenders|extends\\?|false\\?|ffirst|file\\-seq|filter|filterv|find|find\\-keyword|find\\-ns|find\\-protocol\\-impl|find\\-protocol\\-method|find\\-var|first|flatten|float|float\\-array|float\\?|floats|flush|fn\\?|fnext|fnil|force|format|frequencies|future\\-call|future\\-cancel|future\\-cancelled\\?|future\\-done\\?|future\\?|gensym|get|get\\-in|get\\-method|get\\-proxy\\-class|get\\-thread\\-bindings|get\\-validator|group\\-by|halt\\-when|hash|hash\\-combine|hash\\-map|hash\\-ordered\\-coll|hash\\-set|hash\\-unordered\\-coll|ident\\?|identical\\?|identity|ifn\\?|in\\-ns|inc|inc'|indexed\\?|init\\-proxy|inst\\-ms|inst\\-ms\\*|inst\\?|instance\\?|int|int\\-array|int\\?|integer\\?|interleave|intern|interpose|into|into\\-array|ints|isa\\?|iterate|iterator\\-seq|juxt|keep|keep\\-indexed|key|keys|keyword|keyword\\?|last|line\\-seq|list|list\\*|list\\?|load|load\\-file|load\\-reader|load\\-string|loaded\\-libs|long|long\\-array|longs|macroexpand|macroexpand\\-1|make\\-array|make\\-hierarchy|map|map\\-entry\\?|map\\-indexed|map\\?|mapcat|mapv|max|max\\-key|memoize|merge|merge\\-with|meta|method\\-sig|methods|min|min\\-key|mix\\-collection\\-hash|mod|munge|name|namespace|namespace\\-munge|nat\\-int\\?|neg\\-int\\?|neg\\?|newline|next|nfirst|nil\\?|nnext|not|not\\-any\\?|not\\-empty|not\\-every\\?|not=|ns\\-aliases|ns\\-imports|ns\\-interns|ns\\-map|ns\\-name|ns\\-publics|ns\\-refers|ns\\-resolve|ns\\-unalias|ns\\-unmap|nth|nthnext|nthrest|num|number\\?|numerator|object\\-array|odd\\?|parents|partial|partition|partition\\-all|partition\\-by|pcalls|peek|persistent!|pmap|pop|pop!|pop\\-thread\\-bindings|pos\\-int\\?|pos\\?|pr|pr\\-str|prefer\\-method|prefers|primitives\\-classnames|print|print\\-ctor|print\\-dup|print\\-method|print\\-simple|print\\-str|printf|println|println\\-str|prn|prn\\-str|promise|proxy\\-call\\-with\\-super|proxy\\-mappings|proxy\\-name|push\\-thread\\-bindings|qualified\\-ident\\?|qualified\\-keyword\\?|qualified\\-symbol\\?|quot|rand|rand\\-int|rand\\-nth|random\\-sample|range|ratio\\?|rational\\?|rationalize|re\\-find|re\\-groups|re\\-matcher|re\\-matches|re\\-pattern|re\\-seq|read|read+string|read\\-line|read\\-string|reader\\-conditional|reader\\-conditional\\?|realized\\?|record\\?|reduce|reduce\\-kv|reduced|reduced\\?|reductions|ref|ref\\-history\\-count|ref\\-max\\-history|ref\\-min\\-history|ref\\-set|refer|release\\-pending\\-sends|rem|remove|remove\\-all\\-methods|remove\\-method|remove\\-ns|remove\\-tap|remove\\-watch|repeat|repeatedly|replace|replicate|require|requiring\\-resolve|reset!|reset\\-meta!|reset\\-vals!|resolve|rest|restart\\-agent|resultset\\-seq|reverse|reversible\\?|rseq|rsubseq|run!|satisfies\\?|second|select\\-keys|send|send\\-off|send\\-via|seq|seq\\?|seqable\\?|seque|sequence|sequential\\?|set|set\\-agent\\-send\\-executor!|set\\-agent\\-send\\-off\\-executor!|set\\-error\\-handler!|set\\-error\\-mode!|set\\-validator!|set\\?|short|short\\-array|shorts|shuffle|shutdown\\-agents|simple\\-ident\\?|simple\\-keyword\\?|simple\\-symbol\\?|slurp|some|some\\-fn|some\\?|sort|sort\\-by|sorted\\-map|sorted\\-map\\-by|sorted\\-set|sorted\\-set\\-by|sorted\\?|special\\-symbol\\?|spit|split\\-at|split\\-with|str|string\\?|struct|struct\\-map|subs|subseq|subvec|supers|swap!|swap\\-vals!|symbol|symbol\\?|tagged\\-literal|tagged\\-literal\\?|take|take\\-last|take\\-nth|take\\-while|tap>|test|the\\-ns|thread\\-bound\\?|to\\-array|to\\-array\\-2d|trampoline|transduce|transient|tree\\-seq|true\\?|type|unchecked\\-add|unchecked\\-add\\-int|unchecked\\-byte|unchecked\\-char|unchecked\\-dec|unchecked\\-dec\\-int|unchecked\\-divide\\-int|unchecked\\-double|unchecked\\-float|unchecked\\-inc|unchecked\\-inc\\-int|unchecked\\-int|unchecked\\-long|unchecked\\-multiply|unchecked\\-multiply\\-int|unchecked\\-negate|unchecked\\-negate\\-int|unchecked\\-remainder\\-int|unchecked\\-short|unchecked\\-subtract|unchecked\\-subtract\\-int|underive|unquote|unquote\\-splicing|unreduced|unsigned\\-bit\\-shift\\-right|update|update\\-in|update\\-proxy|uri\\?|use|uuid\\?|val|vals|var\\-get|var\\-set|var\\?|vary\\-meta|vec|vector|vector\\-of|vector\\?|volatile!|volatile\\?|vreset!|with\\-bindings\\*|with\\-meta|with\\-redefs\\-fn|xml\\-seq|zero\\?|zipmap)$"))

(anon_fn_lit
 .
 (sym_lit) @function.builtin
 (#match? @function.builtin "^(\\*|\\*'|\\+|\\+'|\\-|\\-'|\\->ArrayChunk|\\->Eduction|\\->Vec|\\->VecNode|\\->VecSeq|\\-cache\\-protocol\\-fn|\\-reset\\-methods|/|<|<=|=|==|>|>=|PrintWriter\\-on|StackTraceElement\\->vec|Throwable\\->map|accessor|aclone|add\\-classpath|add\\-tap|add\\-watch|agent|agent\\-error|agent\\-errors|aget|alength|alias|all\\-ns|alter|alter\\-meta!|alter\\-var\\-root|ancestors|any\\?|apply|array\\-map|aset|aset\\-boolean|aset\\-byte|aset\\-char|aset\\-double|aset\\-float|aset\\-int|aset\\-long|aset\\-short|assoc|assoc!|assoc\\-in|associative\\?|atom|await|await\\-for|await1|bases|bean|bigdec|bigint|biginteger|bit\\-and|bit\\-and\\-not|bit\\-clear|bit\\-flip|bit\\-not|bit\\-or|bit\\-set|bit\\-shift\\-left|bit\\-shift\\-right|bit\\-test|bit\\-xor|boolean|boolean\\-array|boolean\\?|booleans|bound\\-fn\\*|bound\\?|bounded\\-count|butlast|byte|byte\\-array|bytes|bytes\\?|cast|cat|char|char\\-array|char\\-escape\\-string|char\\-name\\-string|char\\?|chars|chunk|chunk\\-append|chunk\\-buffer|chunk\\-cons|chunk\\-first|chunk\\-next|chunk\\-rest|chunked\\-seq\\?|class|class\\?|clear\\-agent\\-errors|clojure\\-version|coll\\?|commute|comp|comparator|compare|compare\\-and\\-set!|compile|complement|completing|concat|conj|conj!|cons|constantly|construct\\-proxy|contains\\?|count|counted\\?|create\\-ns|create\\-struct|cycle|dec|dec'|decimal\\?|dedupe|default\\-data\\-readers|delay\\?|deliver|denominator|deref|derive|descendants|destructure|disj|disj!|dissoc|dissoc!|distinct|distinct\\?|doall|dorun|double|double\\-array|double\\?|doubles|drop|drop\\-last|drop\\-while|eduction|empty|empty\\?|ensure|ensure\\-reduced|enumeration\\-seq|error\\-handler|error\\-mode|eval|even\\?|every\\-pred|every\\?|ex\\-cause|ex\\-data|ex\\-info|ex\\-message|extend|extenders|extends\\?|false\\?|ffirst|file\\-seq|filter|filterv|find|find\\-keyword|find\\-ns|find\\-protocol\\-impl|find\\-protocol\\-method|find\\-var|first|flatten|float|float\\-array|float\\?|floats|flush|fn\\?|fnext|fnil|force|format|frequencies|future\\-call|future\\-cancel|future\\-cancelled\\?|future\\-done\\?|future\\?|gensym|get|get\\-in|get\\-method|get\\-proxy\\-class|get\\-thread\\-bindings|get\\-validator|group\\-by|halt\\-when|hash|hash\\-combine|hash\\-map|hash\\-ordered\\-coll|hash\\-set|hash\\-unordered\\-coll|ident\\?|identical\\?|identity|ifn\\?|in\\-ns|inc|inc'|indexed\\?|init\\-proxy|inst\\-ms|inst\\-ms\\*|inst\\?|instance\\?|int|int\\-array|int\\?|integer\\?|interleave|intern|interpose|into|into\\-array|ints|isa\\?|iterate|iterator\\-seq|juxt|keep|keep\\-indexed|key|keys|keyword|keyword\\?|last|line\\-seq|list|list\\*|list\\?|load|load\\-file|load\\-reader|load\\-string|loaded\\-libs|long|long\\-array|longs|macroexpand|macroexpand\\-1|make\\-array|make\\-hierarchy|map|map\\-entry\\?|map\\-indexed|map\\?|mapcat|mapv|max|max\\-key|memoize|merge|merge\\-with|meta|method\\-sig|methods|min|min\\-key|mix\\-collection\\-hash|mod|munge|name|namespace|namespace\\-munge|nat\\-int\\?|neg\\-int\\?|neg\\?|newline|next|nfirst|nil\\?|nnext|not|not\\-any\\?|not\\-empty|not\\-every\\?|not=|ns\\-aliases|ns\\-imports|ns\\-interns|ns\\-map|ns\\-name|ns\\-publics|ns\\-refers|ns\\-resolve|ns\\-unalias|ns\\-unmap|nth|nthnext|nthrest|num|number\\?|numerator|object\\-array|odd\\?|parents|partial|partition|partition\\-all|partition\\-by|pcalls|peek|persistent!|pmap|pop|pop!|pop\\-thread\\-bindings|pos\\-int\\?|pos\\?|pr|pr\\-str|prefer\\-method|prefers|primitives\\-classnames|print|print\\-ctor|print\\-dup|print\\-method|print\\-simple|print\\-str|printf|println|println\\-str|prn|prn\\-str|promise|proxy\\-call\\-with\\-super|proxy\\-mappings|proxy\\-name|push\\-thread\\-bindings|qualified\\-ident\\?|qualified\\-keyword\\?|qualified\\-symbol\\?|quot|rand|rand\\-int|rand\\-nth|random\\-sample|range|ratio\\?|rational\\?|rationalize|re\\-find|re\\-groups|re\\-matcher|re\\-matches|re\\-pattern|re\\-seq|read|read+string|read\\-line|read\\-string|reader\\-conditional|reader\\-conditional\\?|realized\\?|record\\?|reduce|reduce\\-kv|reduced|reduced\\?|reductions|ref|ref\\-history\\-count|ref\\-max\\-history|ref\\-min\\-history|ref\\-set|refer|release\\-pending\\-sends|rem|remove|remove\\-all\\-methods|remove\\-method|remove\\-ns|remove\\-tap|remove\\-watch|repeat|repeatedly|replace|replicate|require|requiring\\-resolve|reset!|reset\\-meta!|reset\\-vals!|resolve|rest|restart\\-agent|resultset\\-seq|reverse|reversible\\?|rseq|rsubseq|run!|satisfies\\?|second|select\\-keys|send|send\\-off|send\\-via|seq|seq\\?|seqable\\?|seque|sequence|sequential\\?|set|set\\-agent\\-send\\-executor!|set\\-agent\\-send\\-off\\-executor!|set\\-error\\-handler!|set\\-error\\-mode!|set\\-validator!|set\\?|short|short\\-array|shorts|shuffle|shutdown\\-agents|simple\\-ident\\?|simple\\-keyword\\?|simple\\-symbol\\?|slurp|some|some\\-fn|some\\?|sort|sort\\-by|sorted\\-map|sorted\\-map\\-by|sorted\\-set|sorted\\-set\\-by|sorted\\?|special\\-symbol\\?|spit|split\\-at|split\\-with|str|string\\?|struct|struct\\-map|subs|subseq|subvec|supers|swap!|swap\\-vals!|symbol|symbol\\?|tagged\\-literal|tagged\\-literal\\?|take|take\\-last|take\\-nth|take\\-while|tap>|test|the\\-ns|thread\\-bound\\?|to\\-array|to\\-array\\-2d|trampoline|transduce|transient|tree\\-seq|true\\?|type|unchecked\\-add|unchecked\\-add\\-int|unchecked\\-byte|unchecked\\-char|unchecked\\-dec|unchecked\\-dec\\-int|unchecked\\-divide\\-int|unchecked\\-double|unchecked\\-float|unchecked\\-inc|unchecked\\-inc\\-int|unchecked\\-int|unchecked\\-long|unchecked\\-multiply|unchecked\\-multiply\\-int|unchecked\\-negate|unchecked\\-negate\\-int|unchecked\\-remainder\\-int|unchecked\\-short|unchecked\\-subtract|unchecked\\-subtract\\-int|underive|unquote|unquote\\-splicing|unreduced|unsigned\\-bit\\-shift\\-right|update|update\\-in|update\\-proxy|uri\\?|use|uuid\\?|val|vals|var\\-get|var\\-set|var\\?|vary\\-meta|vec|vector|vector\\-of|vector\\?|volatile!|volatile\\?|vreset!|with\\-bindings\\*|with\\-meta|with\\-redefs\\-fn|xml\\-seq|zero\\?|zipmap)$"))

;; anonymous function positional arguments
((sym_lit) @operator
 (#match? @operator "^%"))

;; interop-ish
(list_lit
 .
 (sym_lit) @function.method
 (#match? @function.method "^\\."))

;; quote
(quoting_lit) @constant.character.escape

;; syntax quote
["{" "}" "(" ")" "[" "]"] @punctuation.bracket
["~" "~@" "#'" "@"] @operator
(syn_quoting_lit) @constant.character.escape
