; This roughly follows the description at: https://github.com/ds26gte/scmindent#how-subforms-are-indented

; Exclude literals in the first patterns, since different rules apply for them.
; Similarly, exclude certain keywords (detected by a regular expression).
; If a list has 2 elements on the first line, it is aligned to the second element.
(list . (_) @_fist . (_) @anchor
  (#same-line? @_fist @anchor)
  (#set! "scope" "tail")
  (#not-kind-eq? @_fist "boolean") (#not-kind-eq? @_fist "character") (#not-kind-eq? @_fist "string") (#not-kind-eq? @_fist "number")
  (#not-match? @_fist "def.*|let.*|set!")) @align
; If the first element in a list is also a list and on a line by itself, the outer list is aligned to it
(list . (list) @_first @anchor .
  (#set! @_first  "scope" "tail")
  (#not-kind-eq? @_first "boolean") (#not-kind-eq? @_first "character") (#not-kind-eq? @_first "string") (#not-kind-eq? @_first "number")) @align
(list . (list) @_first @anchor . (_) @_second
  (#not-same-line? @anchor @_second)
  (#set! "scope" "tail")
  (#not-kind-eq? @_first "boolean") (#not-kind-eq? @_first "character") (#not-kind-eq? @_first "string") (#not-kind-eq? @_first "number")
  (#not-match? @_first "def.*|let.*|set!")) @align
; If the first element in a list is not a list and on a line by itself, the outer list is aligned to
; it plus 1 additional space. This cannot currently be modelled exactly by our indent queries,
; but the following is equivalent, assuming that:
; - the indent width is 2 (the default for scheme)
; - There is no space between the opening parenthesis of the list and the first element
(list . (_) @_first .
  (#not-kind-eq? @_first "boolean") (#not-kind-eq? @_first "character") (#not-kind-eq? @_first "string") (#not-kind-eq? @_first "number")
  (#not-match? @_first "def.*|let.*|set!")) @indent
(list . (_) @_first . (_) @_second
  (#not-same-line? @_first @_second)
  (#not-kind-eq? @_first "boolean") (#not-kind-eq? @_first "character") (#not-kind-eq? @_first "string") (#not-kind-eq? @_first "number")
  (#not-match? @_first "def.*|let.*|set!")) @indent

; If the first element in a list is a literal, align the list to it
(list . [(boolean) (character) (string) (number)] @anchor
  (#set! "scope" "tail")) @align

; If the first element is among a set of predefined keywords, align the list to this element
; plus 1 space (using the same workaround as above for now). This is a simplification since actually
; the second line of the list should be indented by 2 spaces more in some cases. Supporting this would
; be possible but require significantly more patterns.
(list . (symbol) @_first
  (#match? @_first "def.*|let.*|set!")) @indent

