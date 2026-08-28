;; Keywords
[
  "CHIP"
  "IN"
  "OUT"
  "PARTS"
  "BUILTIN"
  "CLOCKED"
] @keyword

(identifier) @variable

(chip_definition
  name: (identifier) @function)

(in_section
  input_pin_name: (identifier) @variable.parameter)

(out_section
  output_pin_name: (identifier) @variable.parameter)

(builtin_body
  chip_name: (identifier) @function)

(clocked_body
  (identifier) @variable.parameter)

(part
  chip_name: (identifier) @function)

(connection
  part_pin: (identifier) @variable.other.member
  chip_pin: [
    (identifier) @variable.parameter
    (bus_identifier
      (identifier) @variable.parameter
      (number) @constant.numeric)
  ])

(bus_identifier
  (number) @constant.numeric)

;; Comments
(comment) @comment
