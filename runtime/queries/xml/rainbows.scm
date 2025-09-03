[
  (processing_instructions)
  (cdata_sect)
  (xml_decl)
  (doctype_decl)
  (element_decl)
  (element_choice)
  (element_seq)
  (mixed)
  (attlist_decl)
  (notation_type)
  (enumeration)
  (ge_decl)
  (pe_decl)
  (notation_decl)
] @rainbow.scope

((element) @rainbow.scope
 (#set! rainbow.include-children))

[
  "<?" "?>"
  "<" ">"
  "</" "/>"
  "<!"
  "(" ")"
  ")*"
  "[" "]"
] @rainbow.bracket
