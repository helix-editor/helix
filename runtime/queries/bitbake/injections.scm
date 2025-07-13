((python_function_definition) @injection.content
  (#set! injection.language "python")
  (#set! injection.include-children))

((anonymous_python_function (block) @injection.content)
  (#set! injection.language "python")
  (#set! injection.include-children))

((inline_python) @injection.content
  (#set! injection.language "python")
  (#set! injection.include-children))

((function_definition) @injection.content
  (#set! injection.language "bash")
  (#set! injection.include-children))

((comment) @injection.content
  (#set! injection.language "comment"))
