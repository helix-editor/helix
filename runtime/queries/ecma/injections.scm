; Parse the contents of tagged template literals using
; a language inferred from the tag.

(call_expression
  function: [
    (identifier) @injection.language
    (member_expression
      property: (property_identifier) @injection.language)
  ]
  arguments: (template_string) @injection.content
  (#any-of? @injection.language "html" "css" "json" "sql" "js" "ts" "bash"))

; Parse the contents of $ template literals as shell commands

(call_expression
  function: [
    (identifier) @_template_function_name
    (member_expression
      property: (property_identifier) @_template_function_name)
  ]
  arguments: (template_string) @injection.content
 (#eq? @_template_function_name "$")
 (#set! injection.language "bash"))

; GraphQL detection generally matches the rules provided by the 'GraphQL: Syntax Highlighting'
; VSCode extension: https://github.com/graphql/graphiql/blob/8f25b38f4ab14dc99c046109f255fb283bccde52/packages/vscode-graphql-syntax/grammars/graphql.js.json

; Parse the contents of 'gql' and 'graphql' template literals and function calls
(
  (call_expression
    function: (identifier) @_template_function_name
    arguments: [
      ; Tagged template literal: NAME``
      (template_string (string_fragment) @injection.content)
      (
        arguments . [
          ; Function call containing a string literal: NAME('')
          (string (string_fragment) @injection.content)
          ; Function call containing a template literal: NAME(``)
          (template_string (string_fragment) @injection.content)
        ]
      )
    ]
  )
  (#any-of? @_template_function_name "gql" "graphql")
  (#set! injection.language "graphql")
)

; Parse the contents of strings and tagged template literals that begin with a GraphQL comment '#graphql'
(
  [
    (string (string_fragment) @injection.content)
    (template_string (string_fragment) @injection.content)
  ]
  (#match? @injection.content "^\\s*#graphql")
  (#set! injection.language "graphql")
)

; Parse the contents of strings and tagged template literals with leading ECMAScript comments '/* GraphQL */'
(
  ((comment) @_ecma_comment [
    (string (string_fragment) @injection.content)
    (template_string (string_fragment) @injection.content)
  ])
  (#eq? @_ecma_comment "/* GraphQL */")
  (#set! injection.language "graphql")
)

; Parse regex syntax within regex literals

((regex_pattern) @injection.content
 (#set! injection.language "regex"))

; Parse JSDoc annotations in multiline comments

((comment) @injection.content
 (#set! injection.language "jsdoc")
 (#match? @injection.content "^/\\*+"))

; Parse general tags in single line comments

((comment) @injection.content
 (#set! injection.language "comment")
 (#match? @injection.content "^//"))

; Match string literals passed to standard browser API methods that expects a
; css selector as argument.
; - https://developer.mozilla.org/en-US/docs/Web/API/Document/querySelector
; - https://developer.mozilla.org/en-US/docs/Web/API/Document/querySelectorAll
; - https://developer.mozilla.org/en-US/docs/Web/API/Element/closest
; - https://developer.mozilla.org/en-US/docs/Web/API/Element/matches
; e.g.
; `const el = document.querySelector("div.user-panel.main input[name='login']");`
(call_expression
  function: (member_expression
    object: (identifier) @_object
    property: (property_identifier) @_property (#any-of? @_property "querySelector" "querySelectorAll" "closest" "matches"))
  arguments: (arguments
               (string (string_fragment) @injection.content))
  (#set! injection.language "css"))
