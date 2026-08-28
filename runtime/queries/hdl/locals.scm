; Scopes

(chip_definition) @local.scope

; Definitions

; A chip's IN/OUT/CLOCKED pins are the signals visible inside its PARTS body.
(in_section
  input_pin_name: (identifier) @local.definition.variable.parameter)
(out_section
  output_pin_name: (identifier) @local.definition.variable.parameter)
(clocked_body
  (identifier) @local.definition.variable.parameter)

; References

; In `part_pin=chip_pin`, the chip_pin is a signal in the enclosing chip's scope.
(connection
  chip_pin: (identifier) @local.reference)
(connection
  chip_pin: (bus_identifier
    (identifier) @local.reference))
