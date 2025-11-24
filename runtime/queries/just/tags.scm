; Symbols that can be considered definitions in a Just file.

(alias
  alias_name: (identifier) @definition.function)

(assignment
  name: (identifier) @definition.constant)

(import
  (path) @definition.module)

(mod
  name: (identifier) @definition.module)

(recipe
  name: (identifier) @definition.function)

(unexport
  name: (identifier) @definition.constant)
