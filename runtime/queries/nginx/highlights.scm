(comment) @comment

(number) @constant.numeric
(metric) @constant.numeric

(regex) @string

(variable) @variable

(modifier) @keyword.operator

(simple_directive
	name: (directive) @function)

(block_directive
	name: (directive) @function)

(lua_block_directive
	"access_by_lua_block" @function)

((generic) @constant.builtin
	(#match? @constant.builtin "^(off|on)$"))

(generic) @string
(string) @string

(scheme) @string
(ipv4) @constant.numeric

[
	";"
] @delimiter

[
	"{"
	"}"
	"("
	")"
	"["
	"]"
] @punctuation.bracket

; Lua Debug
(lua_code) @definition.type
