; tags and labels
(label) @label
(tag (identifier) @commment)
(tag) @comment

; values
(identifier) @function
(string) @string
(boolean) @constant
(number) @constant.numeric

; headers
(knot_header) @keyword
(stitch_header) @keyword
(function_header) @keyword

; marks (ink)
(option_mark) @keyword.directive
(gather_mark) @type.builtin
(glue) @type.builtin

; calls
(divert_or_thread) @function

; operators
(assignment) @operator

; special marks/operators (ink)
(arrow) @special
(double_arrow) @special
(back_arrow) @constant
(dot) @special
(mark_start) @special
(mark_end) @special
(hide_start) @special
(hide_end) @special

; declarations
(var_line) @attribute
(const_line) @constant
(list_line) @type

; comments
(line_comment) @comment
(block_comment) @comment

; unparsed code
(inline_block) @keyword
(condition_block) @keyword
(code_text) @keyword

; support injection
(program) @ui.text
