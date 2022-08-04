(date) @variable.builtin
(txn) @variable.builtin

(account) @type

[
  (amount)
  (incomplete_amount)
  (amount_tolerance)
  (number)
] @constant.numeric


(key) @label
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
] @keyword.operator

[
  (balance) (open) (close) (commodity) (pad)
  (event) (price) (note) (document) (query)
  (custom) (pushtag) (poptag) (pushmeta)
  (popmeta) (option) (include) (plugin)
] @keyword
