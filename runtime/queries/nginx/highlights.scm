(comment) @comment

(block_directive
	(directive) @type)

[
	"{"
	"}"
	"("
	")"
	"["
	"]"
] @punctuation.bracket

(simple_directive
	(directive) @function)

[
	";"
] @punctuation.delimiter

((generic) @keyword
 (#any-of? @keyword
 	"on"
 	"off"
 	"any"
 	"auto"))

(modifier) @operator

(generic) @variable

(string) @string

(number) @constant.numeric
(metric) @constant.numeric

(variable) @variable.parameter

(regex) @string

(modifier) @keyword.operator

(lua_block_directive
	"access_by_lua_block" @function)
