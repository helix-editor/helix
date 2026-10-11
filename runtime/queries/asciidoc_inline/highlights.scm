[
  (monospace)
  (passthrough)
] @nospell @markup.raw.inline

(emphasis) @markup.bold

(italic) @markup.italic

(highlight) @markup.bold

(superscript) @markup.italic

(subscript) @markup.italic

[
  (link_url)
  (email)
] @markup.link @markup.link.url

(uri_label) @markup.link.label

[
  "["
  "]"
  "{"
  "}"
  "<<" @punctuation.bracket
  ">>" @punctuation.bracket
] @punctuation.bracket

":" @punctuation.delimiter

(replacement) @string.special

(roled_text
  (role) @attribute)

(attribute_reference
  (attribute_name) @constant)

(xref
  (id) @markup.link.label
  (reftext)? @markup.link.text)

[
  (macro_name)
  "((("
  ")))"
  "(("
  "))"
] @function.macro

(escaped_sequence) @string.escape

(inline_macro
  (macro_name) @function.macro
  (target)? @label
  (attr)? @label)

((inline_macro
  (macro_name) @function.macro
  (target) @markup.link.url
  (attr)? @markup.link.text)
  (#any-of? @function.macro "link" "mailto"))

(stem_macro
  (target)? @label
  (attr)? @nospell @markup.raw.inline)

(footnote
  (target)? @label
  (attr) @attribute)

; The value of a named macro attribute (e.g. `window=_blank`).
(named_attr
  (attribute_value) @string)

(term) @attribute

(id_assignment) @label

(super_escape) @string.special

(hard_wrap) @punctuation.special
