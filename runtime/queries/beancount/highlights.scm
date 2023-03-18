(date) @variable.builtin
(txn) @variable.builtin

(account) @type

[
  (amount)
  (incomplete_amount)
  (amount_tolerance)
  (number)
] @constant.numeric


[(key_value) (key)] @variable.other.member
(string) @string

[
  (currency)
  (tag)
  (link)
] @constant

(comment) @comment

[
  (minus)
  (plus)
] @operator

[
  (balance) (open) (close) (commodity) (pad)
  (event) (price) (note) (document) (query)
  (custom) (pushtag) (poptag) (pushmeta)
  (popmeta) (option) (include) (plugin)
] @keyword


((headline item: (item) @markup.heading.6) @markup.heading.marker
 (#match? @markup.heading.marker "^\\*\\*\\*\\*\\*\\*"))
((headline item: (item) @markup.heading.5) @markup.heading.marker
 (#match? @markup.heading.marker "^\\*\\*\\*\\*\\*"))
((headline item: (item) @markup.heading.4) @markup.heading.marker
 (#match? @markup.heading.marker "^\\*\\*\\*\\*"))
((headline item: (item) @markup.heading.3) @markup.heading.marker
 (#match? @markup.heading.marker "^\\*\\*\\*"))
((headline item: (item) @markup.heading.2) @markup.heading.marker
 (#match? @markup.heading.marker "^\\*\\*"))
((headline item: (item) @markup.heading.1) @markup.heading.marker
 (#match? @markup.heading.marker "^\\*"))
