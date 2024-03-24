(comment) @comment

[ "(" ")" "{" "}" "[" "]" ] @punctuation.bracket

[ ":" ":until" "&" "&as" "?" ] @punctuation.special

(nil) @constant.builtin
(vararg) @punctuation.special

(boolean) @constant.builtin.boolean
(number) @constant.numeric

(string) @string
(escape_sequence) @constant.character.escape


((symbol) @variable.builtin
 (#match? @variable.builtin "^[$]"))

(binding) @symbol

[ "fn" "lambda" "hashfn" "#" ] @keyword.function

(fn name: [
  (symbol) @function
  (multi_symbol (symbol) @function .)
])

(lambda name: [
  (symbol) @function
  (multi_symbol (symbol) @function .)
])

(multi_symbol
  "." @punctuation.delimiter
  (symbol) @variable.other.member)

(multi_symbol_method
  ":" @punctuation.delimiter
  (symbol) @function.method .)

[ "for" "each" ] @keyword.control.repeat
((symbol) @keyword.control.repeat
 (#eq? @keyword.control.repeat
  "while"))

[ "match" ] @keyword.control.conditional
((symbol) @keyword.control.conditional
 (#match? @keyword.control.conditional "^(if|when)$"))

[ "global" "local" "let" "set" "var" "where" "or" ] @keyword
((symbol) @keyword
 (#match? @keyword
  "^(comment|do|doc|eval-compiler|lua|macros|quote|tset|values)$"))

((symbol) @keyword.control.import
 (#match? @keyword.control.import
  "^(require|require-macros|import-macros|include)$"))

[ "collect" "icollect" "accumulate" ] @function.macro
((symbol) @function.macro
 (#match? @function.macro
  "^(->|->>|-\\?>|-\\?>>|\\?\\.|doto|macro|macrodebug|partial|pick-args|pick-values|with-open)$"))

; Lua builtins
((symbol) @constant.builtin
 (#match? @constant.builtin
  "^(arg|_ENV|_G|_VERSION)$"))

((symbol) @function.builtin
 (#match? @function.builtin
  "^(assert|collectgarbage|dofile|error|getmetatable|ipairs|load|loadfile|loadstring|module|next|pairs|pcall|print|rawequal|rawget|rawlen|rawset|require|select|setfenv|setmetatable|tonumber|tostring|type|unpack|warn|xpcall)$"))

(list . (symbol) @function)
(list . (multi_symbol (symbol) @function .))
(symbol) @variable
