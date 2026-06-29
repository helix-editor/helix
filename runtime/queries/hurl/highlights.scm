[
  "[QueryStringParams]"
  "[Query]"
  "[FormParams]"
  "[Form]"
  "[MultipartFormData]"
  "[Multipart]"
  "[Cookies]"
  "[Captures]"
  "[Asserts]"
  "[Options]"
  "[BasicAuth]"
] @attribute

(comment) @comment

[
  (key_string)
  (json_key_string)
] @variable.other.member
 
(value_string) @string
(quoted_string) @string
(json_string) @string
(file_value) @string.special.path
(regex) @string.regexp

[
  "\\"
  (regex_escaped_char)
  (quoted_string_escaped_char)
  (key_string_escaped_char)
  (value_string_escaped_char)
  (oneline_string_escaped_char)
  (multiline_string_escaped_char)
  (filename_escaped_char)
  (json_string_escaped_char)
] @constant.character.escape

(method) @type.builtin
(multiline_string_type) @type

[
  "status"
  "url"
  "header"
  "cookie"
  "body"
  "xpath"
  "jsonpath"
  "regex"
  "variable"
  "duration"
  "sha256"
  "md5"
  "bytes"
  "daysAfterNow"
  "daysBeforeNow"
  "htmlEscape"
  "htmlUnescape"
  "decode"
  "format"
  "nth"
  "replace"
  "split"
  "toDate"
  "toInt"
  "urlEncode"
  "urlDecode"
  "count"
] @function.builtin

(filter) @attribute

(version) @string.special
"null" @constant.builtin

; Option keys (location, max-time, retry, cert, user, … and the many added in
; newer hurl): the grammar generalised per-option nodes into boolean/integer/
; string/duration options with an `option_key` field — capture that field so
; every option key is covered uniformly instead of listing each by name.
(_ option_key: _ @constant.builtin)

(boolean) @constant.builtin.boolean

(variable_name) @variable

[
  "not"
  "equals"
  "=="
  "notEquals"
  "!="
  "greaterThan"
  ">"
  "greaterThanOrEquals"
  ">="
  "lessThan"
  "<"
  "lessThanOrEquals"
  "<="
  "startsWith"
  "endsWith"
  "contains"
  "matches"
  "exists"
  "includes"
  "isInteger"
  "isFloat"
  "isBoolean"
  "isString"
  "isCollection"
  "isNumber"
  "isIsoDate"
  "isEmpty"
] @keyword.operator

(integer) @constant.numeric.integer
(float) @constant.numeric.float
(status) @constant.numeric
(json_number) @constant.numeric.float

[
  ":"
  ","
] @punctuation.delimiter

[
  "["
  "]"
  "{"
  "}"
  "{{"
  "}}"
] @punctuation.special

[
  "base64,"
  "file,"
  "hex,"
] @string.special
