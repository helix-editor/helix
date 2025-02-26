; header fields
[
  (header_field_email)
  (header_field_subject)
  (header_field)
] @keyword

; delimited punctuation
(header_separator) @punctuation.delimiter
(email_delimiter) @punctuation.delimiter

; email subject contents
(header_subject
  (subject) @markup.bold)
; extra metadata headers
(header_other
  (header_unstructured) @comment)

; Addressee Name (Firstname, Lastname, etc.)
(atom) @variable

; Email Address
(email) @string

; Quoted Reply
(quote_marker) @punctuation.special
(quote_contents) @markup.quote

