; extends
; Comments
(comment) @comment

; Keys
(property) @variable

; Values
(boolean) @constant.builtin.boolean


[
 (number)
 (adjustment)
] @constant.numeric

[
 "+"
 "="
 (keybind_trigger ">")
] @operator

(":") @punctuation.delimiter

[
 (string)
 (color)
] @string

; (color) are hex values
(color "#" @punctuation.special
 (#eq? @punctuation.special "#"))

(path_value "?" @keyword.control.conditional
    (#eq? @keyword.control.conditional "?"))

; `palette`
(palette_index) @variable.other.member

; `path_directive`
(path_directive (property) @keyword.import)
(path_directive (path_value (string) @string.special.path ))


(action_name) @function.builtin
(action_argument (string) @variable.parameter ) 

; (tuple)
(tuple "," @punctuation.delimiter.special
       (#eq? @punctuation.delimiter.special ","))

; `keybind`
(keybind_value) @string.special

; clear is a special keyword that clear all existing keybind up to that point
((keybind_value) @keyword 
 (#eq? @keyword "clear"))

; NOTE: The order here matters!
[
 (key_qualifier)
 (keybind_modifier)
] @attribute

[
 (modifier_key)
 (key)
] @constant.builtin
