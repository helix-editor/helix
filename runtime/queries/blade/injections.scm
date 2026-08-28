; inherits: html

; tree-sitter-comment injection
; if available
((comment) @injection.content
 (#set! injection.language "comment"))

((php_only) @injection.content
    (#set! injection.language "php-only"))

((parameter) @injection.content
    (#set! injection.include-children) ; You may need this, depending on your editor e.g Helix
    (#set! injection.language "php-only"))

; ; Livewire attributes
(attribute
  (attribute_name) @_attr
    (#any-of? @_attr
      "wire:click"
      "wire:submit"
      "wire:model"
      "wire:loading"
      "wire:navigate"
      "wire:current"
      "wire:cloak"
      "wire:dirty"
      "wire:confirm"
      "wire:transition"
      "wire:init"
      "wire:poll"
      "wire:offline"
      "wire:ignore"
      "wire:replace"
      "wire:show"
      "wire:stream"
      "wire:text"
    )
  (quoted_attribute_value
    (attribute_value) @injection.content)
  (#set! injection.language "javascript"))

; ; See #33
; ; AlpineJS attributes
(attribute
  (attribute_name) @_attr
    (#match? @_attr "^x-[a-z]+")
  (quoted_attribute_value
    (attribute_value) @injection.content)
  (#set! injection.language "javascript"))

; ; Apline Events
(attribute
  (attribute_name) @_attr
    (#match? @_attr "^@[a-z]+")
  (quoted_attribute_value
    (attribute_value) @injection.content)
  (#set! injection.language "javascript"))

; ; normal HTML element alpine attributes
(element
  (_
    (tag_name) @_tag
      (#match? @_tag "[^x][^-]")
    (attribute
      (attribute_name) @_attr
        (#match? @_attr "^:[a-z]+")
      (quoted_attribute_value
        (attribute_value) @injection.content)
      (#set! injection.combined)
      (#set! injection.language "javascript"))))

; ; ; Blade escaped JS attributes
; ; <x-foo ::bar="baz" />
(element
  (_
    (tag_name) @_tag
      (#match? @_tag "^x-[a-z]+")
    (attribute
      (attribute_name) @_attr
        (#match? @_attr "^::[a-z]+")
      (quoted_attribute_value
        (attribute_value) @injection.content)
      (#set! injection.language "javascript"))))


; ; ; Blade escaped JS attributes
; ; <htmlTag :class="baz" />
(element
  (_
    (attribute_name) @_attr
      (#match? @_attr "^:[a-z]+")
    (quoted_attribute_value
      (attribute_value) @injection.content)
    (#set! injection.language "javascript")))


; Blade PHP attributes
(element
  (_
    (tag_name) @_tag
      (#match? @_tag "^x-[a-z]+")
    (attribute
      (attribute_name) @_attr
        (#match? @_attr "^:[a-z]+")
      (quoted_attribute_value
        (attribute_value) @injection.content)
      (#set! injection.language "php-only"))))

