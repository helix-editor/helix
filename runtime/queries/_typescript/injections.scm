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
          (template_string) @injection.content
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
          (template_string) @injection.content
          (array
            (string) @injection.content)
          (array
            (template_string) @injection.content)
        ])))
  (#set! injection.language "css"))
