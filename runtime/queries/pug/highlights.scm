(comment) @comment

(
  doctype
  (("doctype") @keyword.storage.type)
  ((doctype_name) @type.enum.variant)
)

(tag_name) @constant

; Attributes
(id) @attribute
(class) @attribute
(attribute_name) @attribute

(quoted_attribute_value) @string

; Controls
(
  conditional
  ((keyword) @keyword.control.conditional)
)
(
  case
  ((keyword) @keyword.control)
  (
    when
    ((keyword) @keyword.control)
  )
)
(
  each
  ((keyword) @keyword.control.repeat)
)
(
  else
  ((keyword) @keyword.control.conditional)
)
(
  while
  ((keyword) @keyword.control.repeat)
)

; Mixins
(
  mixin_definition
  ((keyword) @keyword.function)
  ((mixin_name) @function.method)
)
(
  mixin_use
  (("+") @operator)
  ((mixin_name) @function.method)
)

; Includes
(
  include
  ((keyword) @keyword.directive)
  ((filename) @string.special.path)
)

; Inheritance
(
  extends
  ((keyword) @keyword.directive)
  ((filename) @string.special.path)
)
(
  block_definition
  ((keyword) @keyword.directive)
  ((block_name) @function.method)
)
(
  block_append
  ((keyword) @keyword.directive)
  ((block_name) @function.method)
)
(
  block_prepend
  ((keyword) @keyword.directive)
  ((block_name) @function.method)
)

; Filters
(
  filter
  (":" @function.macro)
  ((filter_name) @function.macro)
  ((content) @special)
)

; Inline JavaScript
(
  unbuffered_code
  (("-") @special)
)
