[
  "[QueryStringParams]"
  "[FormParams]"
  "[MultipartFormData]"
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
(regex) @string.regex

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
[
  "null"
  "cacert"
  "compressed"
  "location"
  "insecure"
  "path-as-is"
  "proxy"
  "max-redirs"
  "retry"
  "retry-interval"
  "retry-max-count"
  (variable_option "variable")
  "verbose"
  "very-verbose"
] @constant.builtin

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
