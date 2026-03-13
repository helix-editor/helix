; match/exclude regex with regular string
(node
  (identifier) @_section_name
  (#any-of? @_section_name "window-rule" "layer-rule")
  children: (node_children
    (node
      (identifier) @_node_name
      (#any-of? @_node_name "match" "exclude")
      (node_field
        (prop
          (identifier) @_prop_name
          (#any-of? @_prop_name "app-id" "title" "namespace")
          (value
            (string
              (string_fragment) @injection.content
              (#set! injection.language "regex")
            )
          )
        )
      )
    )
  )
)

(node
  (identifier) @_section
  (#eq? @_section "binds")
  children: (node_children
    (node
      (identifier)
      children: (node_children
        (node
          (identifier) @_action_name
          (#eq? @_action_name "spawn")
          (node_field
            (value
              (string
                (string_fragment) @_executable
                (#eq? @_executable "fish")
              )
            )
          )
          (node_field
            (value
              (string
                (string_fragment) @_flag
                (#eq? @_flag "-c")
              )
            )
          )
          (node_field
            (value
              (string
                (string_fragment) @injection.content
                (#set! injection.language "fish")
              )
            )
          )
        )
      )
    )
  )
)
