; This roughly follows the description at: https://github.com/ds26gte/scmindent#how-subforms-are-indented

; Exclude literals in the first patterns, since different rules apply for them.
; Similarly, exclude certain keywords (detected by a regular expression).
; If a list has 2 elements on the first line, it is aligned to the second element.
(list . (_) @first . (_) @anchor
  (#same-line? @first @anchor)
  (#set! "scope" "tail")
  (#not-kind-eq? @first "boolean") (#not-kind-eq? @first "character") (#not-kind-eq? @first "string") (#not-kind-eq? @first "number")
  (#not-match? @first "def.*|let.*|set!")) @align
; If the first element in a list is also a list and on a line by itself, the outer list is aligned to it
(list . (list) @anchor .
  (#set! "scope" "tail")
  (#not-kind-eq? @first "boolean") (#not-kind-eq? @first "character") (#not-kind-eq? @first "string") (#not-kind-eq? @first "number")) @align
(list . (list) @anchor . (_) @second
  (#not-same-line? @anchor @second)
  (#set! "scope" "tail")
  (#not-kind-eq? @first "boolean") (#not-kind-eq? @first "character") (#not-kind-eq? @first "string") (#not-kind-eq? @first "number")
  (#not-match? @first "def.*|let.*|set!")) @align
; If the first element in a list is not a list and on a line by itself, the outer list is aligned to
; it plus 1 additional space. This cannot currently be modelled exactly by our indent queries,
; but the following is equivalent, assuming that:
; - the indent width is 2 (the default for scheme)
; - There is no space between the opening parenthesis of the list and the first element
(list . (_) @first .
  (#not-kind-eq? @first "boolean") (#not-kind-eq? @first "character") (#not-kind-eq? @first "string") (#not-kind-eq? @first "number")
  (#not-match? @first "def.*|let.*|set!")) @indent
(list . (_) @first . (_) @second
  (#not-same-line? @first @second)
  (#not-kind-eq? @first "boolean") (#not-kind-eq? @first "character") (#not-kind-eq? @first "string") (#not-kind-eq? @first "number")
  (#not-match? @first "def.*|let.*|set!")) @indent

; If the first element in a list is a literal, align the list to it
(list . [(boolean) (character) (string) (number)] @anchor
  (#set! "scope" "tail")) @align

; If the first element is among a set of predefined keywords, align the list to this element
; plus 1 space (using the same workaround as above for now). This is a simplification since actually
; the second line of the list should be indented by 2 spaces more in some cases. Supporting this would
; be possible but require significantly more patterns.
(list . (symbol) @first
  (#match? @first "def.*|let.*|set!")) @indent

