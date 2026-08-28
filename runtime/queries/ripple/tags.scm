; Code outline/structure for symbol navigation

; Components
(component_declaration
  name: (identifier) @name) @definition.function

; Fragments
(fragment_declaration
  name: (identifier) @name) @definition.function

; Functions
(function_declaration
  name: (identifier) @name) @definition.function

; Classes
(class_declaration
  name: (identifier) @name) @definition.class

; Methods
(method_definition
  name: (property_name) @name) @definition.function
