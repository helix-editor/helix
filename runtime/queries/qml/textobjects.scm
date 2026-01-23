; (comment) is used for both // and /* ... */ comment syntax
(comment) @comment.inside
(comment)+ @comment.around

(ui_object_definition
  initializer: (_) @class.inside) @class.around

(ui_binding
  name: (identifier) @parameter.inside) @parameter.around

(ui_property
  (_)+ @parameter.inside ":") @parameter.around

(function_declaration
  body: (_) @function.inside) @function.around

(arrow_function
  body: (_) @function.inside) @function.around

; e.g. `onClicked: console.log("Button clicked!")`
((ui_binding
  name: (identifier) @_name
  value: (_) @function.around @function.inside)
  (#match? @_name "^on[A-Z].*"))

; e.g.
; Component.onCompleted: {
;    console.log("completed")
; }
(statement_block (expression_statement)* @function.inside) @function.around

; e.g.
; states: [
;        State { name: "activated" },
;        State { name: "deactivated" }
; ]
(ui_object_array
  ((_) @entry.inside . ","? @entry.around) @entry.around)

; e.g. [1, 2, 3, 4]
(array
  ((_) @entry.inside . ","? @entry.around) @entry.around)

; Tests in QML are written using "Qt Quick Test" and it's `TestCase` type
; ref: https://doc.qt.io/qt-6/qtquicktest-index.html
((ui_object_definition
  type_name: (identifier) @_name
  initializer: (_) @test.inside) @test.around
  (#eq? @_name "TestCase"))
