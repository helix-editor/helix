; Inject Angular HTML inside @Component({ template: ... })
(call_expression
  function: [
    (identifier) @_decorator
    (#eq? @_decorator "Component")
    (member_expression
      property: (property_identifier) @_decorator
      (#eq? @_decorator "Component"))
  ]
  arguments: (arguments
    (object
      (pair
        key: (property_identifier) @_prop
        (#eq? @_prop "template")
        value: [
          (string) @injection.content
          (template_string (string_fragment) @injection.content)
          (string_fragment) @injection.content
        ])))
  (#set! injection.language "angular"))

; Angular Component styles injection
(call_expression
  function: [
    (identifier) @_decorator
    (#eq? @_decorator "Component")
    (member_expression
      property: (property_identifier) @_decorator
      (#eq? @_decorator "Component"))
  ]
  arguments: (arguments
    (object
      (pair
        key: (property_identifier) @_prop
        (#eq? @_prop "styles")
        value: [
          (string) @injection.content
          (template_string (string_fragment) @injection.content)
          (string_fragment) @injection.content
          (array (template_string (string_fragment) @injection.content))
          (array (string) @injection.content)
        ])))
  (#set! injection.language "css"))

