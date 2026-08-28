(section
  (identifier) @_type
  (attribute
    (identifier) @_attr_type
    (string) @definition.struct)
  (#eq? @_type "node")
  (#eq? @_attr_type "name"))

(section
  (identifier) @definition.struct
  (#not-eq? @definition.struct "node"))
