; Code outline/structure for symbol navigation

; Components
(component_declaration
  name: (identifier) @name) @item

; Fragments
(fragment_declaration
  name: (identifier) @name) @item

; Functions
(function_declaration
  name: (identifier) @name) @item

; Classes
(class_declaration
  name: (identifier) @name) @item

; Methods
(method_definition
  name: (property_name) @name) @item

; Variables (const/let)
(variable_declarator
  name: (identifier) @name) @item
