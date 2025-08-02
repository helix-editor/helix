;; Keywords
[
  "CHIP"
  "IN"
  "OUT"
  "PARTS"
] @keyword

(identifier) @variable

(chip_definition
  name: (identifier) @function)

(in_section
  input_pin_name: (identifier) @variable.parameter)

(out_section
  output_pin_name: (identifier) @variable.parameter)

(part
  chip_name: (identifier) @function)

(connection
  part_pin: (identifier) @variable.other.member
  chip_pin: (identifier) @variable.parameter)

;; Comments
(comment) @comment
