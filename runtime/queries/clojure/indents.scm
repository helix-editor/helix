; If a list has 2 elements on the first line, align to the second element.
; Exclude literals and special keywords that have different indentation rules.
(list_lit . (_) @_first . (_) @anchor
  (#same-line? @_first @anchor)
  (#set! "scope" "tail")
  (#not-kind-eq? @_first "bool_lit")
  (#not-kind-eq? @_first "nil_lit")
  (#not-kind-eq? @_first "str_lit")
  (#not-kind-eq? @_first "num_lit")
  (#not-kind-eq? @_first "kwd_lit")
  (#not-match? @_first "^(def|defn|defn-|defmacro|defmethod|defmulti|defonce|defprotocol|deftype|defrecord|defstruct|definline|definterface|deftest|ns|let|letfn|binding|loop|for|doseq|dotimes|when-let|if-let|when-some|if-some|with-.*|fn)$")) @align

; If the first element in a list is also a list and on a line by itself,
; the outer list is aligned to it
(list_lit . (list_lit) @anchor .
  (#set! "scope" "tail")) @align

(list_lit . (list_lit) @anchor . (_) @_second
  (#not-same-line? @anchor @_second)
  (#set! "scope" "tail")) @align

; If the first element in a list is not a list and on a line by itself,
; indent the list body by one level
(list_lit . (_) @_first .
  (#not-kind-eq? @_first "bool_lit")
  (#not-kind-eq? @_first "nil_lit")
  (#not-kind-eq? @_first "str_lit")
  (#not-kind-eq? @_first "num_lit")
  (#not-kind-eq? @_first "kwd_lit")
  (#not-match? @_first "^(def|defn|defn-|defmacro|defmethod|defmulti|defonce|defprotocol|deftype|defrecord|defstruct|definline|definterface|deftest|ns|let|letfn|binding|loop|for|doseq|dotimes|when-let|if-let|when-some|if-some|with-.*|fn)$")) @indent

(list_lit . (_) @_first . (_) @_second
  (#not-same-line? @_first @_second)
  (#not-kind-eq? @_first "bool_lit")
  (#not-kind-eq? @_first "nil_lit")
  (#not-kind-eq? @_first "str_lit")
  (#not-kind-eq? @_first "num_lit")
  (#not-kind-eq? @_first "kwd_lit")
  (#not-match? @_first "^(def|defn|defn-|defmacro|defmethod|defmulti|defonce|defprotocol|deftype|defrecord|defstruct|definline|definterface|deftest|ns|let|letfn|binding|loop|for|doseq|dotimes|when-let|if-let|when-some|if-some|with-.*|fn)$")) @indent

; If the first element is a literal, align the list to it
(list_lit . [(bool_lit) (nil_lit) (str_lit) (num_lit) (kwd_lit)] @anchor
  (#set! "scope" "tail")) @align

; Special indentation for def-like forms, let bindings, and other special forms
; These forms typically have the body indented by one level after the name/bindings
(list_lit . (sym_lit) @_first
  (#match? @_first "^(def|defn|defn-|defmacro|defmethod|defmulti|defonce|defprotocol|deftype|defrecord|defstruct|definline|definterface|deftest|ns|let|letfn|binding|loop|for|doseq|dotimes|when-let|if-let|when-some|if-some|with-.*|fn)$")) @indent

; Align vector/map elements when first two are on same line (e.g., let bindings)
(vec_lit . (_) @anchor . (_) @_second
  (#same-line? @anchor @_second)
  (#set! "scope" "tail")) @align

(map_lit . (_) @anchor . (_) @_second
  (#same-line? @anchor @_second)
  (#set! "scope" "tail")) @align

; Indent vectors, maps, and sets
[(vec_lit) (map_lit) (set_lit)] @indent
