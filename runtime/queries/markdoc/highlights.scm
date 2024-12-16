tag_name: (identifier) @tag
(tag_self_closing "/" @tag)
(tag_close "/" @tag)
([(tag_start) (tag_end) "="] @tag)
(attribute [key : (identifier)] @attribute)
(attribute [shorthand : (identifier)]  @attribute)
(variable [variable : (identifier) (variable_sigil)] @variable)
(variable_tail property : (identifier) @variable.other.member)
(function function_name : (identifier) @function)
(function_parameter_named parameter : (identifier) @variable.parameter)

(hash_key key: (identifier) @variable.other.member)
(string) @string
(number) @constant.numeric
(boolean) @constant.builtin.boolean
(null) @constant.builtin
