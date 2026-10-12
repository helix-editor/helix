(ERROR)          @error
(malformed_line) @error

(address_field                    name: (field_name) @constant)
(date_field                       name: (field_name) @constant)
(message_id_field                 name: (field_name) @constant)
(in_reply_to_field                name: (field_name) @constant)
(references_field                 name: (field_name) @constant)
(received_field                   name: (field_name) @constant)
(return_path_field                name: (field_name) @constant)
(subject_field                    name: (field_name) @constant)
(comments_field                   name: (field_name) @constant)
(content_type_field               name: (field_name) @constant)
(content_transfer_encoding_field  name: (field_name) @constant)
(content_disposition_field        name: (field_name) @constant)
(mime_version_field               name: (field_name) @constant)
(content_description_field        name: (field_name) @constant)

; Covers non-ASCII field names as well, which the grammar degrades to
; optional_field
(optional_field name: (field_name) @property)

(subject_field value: (unstructured) @markup.heading)
(subject_field prefix: (reply_forward_marker) @comment)

(comments_field            value: (unstructured) @string)
(content_description_field value: (unstructured) @string)
(optional_field            value: (unstructured) @string)
(content_type_field        value: (unstructured) @string)
(mime_version_field        value: (unstructured) @string)

; Obsolete forms (2-digit year, named/military timezones like GMT) share
; the same `year`/`zone` node types as their modern equivalents.
(date_time)   @string.special
(day_of_week) @string.special
(day)         @string.special
(month)       @string.special
(year)        @string.special
(hour)        @string.special
(minute)      @string.special
(second)      @string.special
(zone)        @string.special

(msg_id) @string.special
(msg_id "<" @punctuation.bracket)
(msg_id ">" @punctuation.bracket)
(msg_id "@" @punctuation.delimiter)

; addr_spec (local@domain) captured without surrounding <> brackets.
(addr_spec) @markup.link.url
(addr_spec "@" @punctuation.delimiter)

(domain_literal) @string.special

(angle_addr "<" @punctuation.bracket)
(angle_addr ">" @punctuation.bracket)

(mailbox display_name: (display_name) @string)
(group   display_name: (display_name) @string)

(group ":" @punctuation.delimiter)
(group ";" @punctuation.delimiter)

(encoded_word) @constant

(mailbox_sep "," @punctuation.delimiter)

(route) @string.special

(return_path_value) @string.special

(mime_type)    @type
(mime_subtype) @type
(content_type "/" @punctuation.delimiter)

(parameter_name)  @property
(parameter_value) @string

; The quoted form (e.g. filename="report.pdf") needs its own capture.
(mime_parameter value: (quoted_string) @string)
(mime_parameter          "=" @punctuation.delimiter)
(mime_parameter          ";" @punctuation.special)
(mime_boundary_parameter "=" @punctuation.delimiter)
(mime_boundary_parameter ";" @punctuation.special)

(encoding_mechanism) @string.special
(disposition_type)   @string.special
(mime_version)       @string.special

; Highlight attachment filenames distinctly.
(content_disposition
  type: (disposition_type) @_disposition_type
  (mime_parameter
    name: (parameter_name) @_param_name
    value: (_) @label)
  (#match? @_disposition_type "^[Aa][Tt][Tt][Aa][Cc][Hh][Mm][Ee][Nn][Tt]$")
  (#match? @_param_name "^[Ff][Ii][Ll][Ee][Nn][Aa][Mm][Ee]"))

(dash_boundary)   @punctuation.special
(close_delimiter) @punctuation.special

; Base case: every quote marker/line gets markup.quote regardless of depth.
(quote_marker)   @markup.quote
(quote_contents) @markup.quote

; Apply progressively deeper quote captures through six nesting levels.
(quote_group
  (quoted_block
    [(quote_marker) (quote_contents)] @markup.quote.1))

(quote_group
  (quoted_block
    (quoted_block
      [(quote_marker) (quote_contents)] @markup.quote.2)))

(quote_group
  (quoted_block
    (quoted_block
      (quoted_block
        [(quote_marker) (quote_contents)] @markup.quote.3))))

(quote_group
  (quoted_block
    (quoted_block
      (quoted_block
        (quoted_block
          [(quote_marker) (quote_contents)] @markup.quote.4)))))

(quote_group
  (quoted_block
    (quoted_block
      (quoted_block
        (quoted_block
          (quoted_block
            [(quote_marker) (quote_contents)] @markup.quote.5))))))

(quote_group
  (quoted_block
    (quoted_block
      (quoted_block
        (quoted_block
          (quoted_block
            (quoted_block
              [(quote_marker) (quote_contents)] @markup.quote.6)))))))

(signature_separator) @punctuation.special
(signature)           @comment

(comment) @comment

(received_tokens) @string
