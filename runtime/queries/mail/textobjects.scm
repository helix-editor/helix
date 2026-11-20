(address_field
  value: (address_list) @field.inner @entry.inside) @field.outer @entry.around

(subject_field
  value: (unstructured) @field.inner @entry.inside) @field.outer @entry.around

(comments_field
  value: (unstructured) @field.inner @entry.inside) @field.outer @entry.around

(date_field
  value: (_) @field.inner @entry.inside) @field.outer @entry.around

(message_id_field
  value: (msg_id) @field.inner @entry.inside) @field.outer @entry.around

(in_reply_to_field) @field.outer @entry.around
(references_field)  @field.outer @entry.around

(received_field) @field.outer @entry.around

(return_path_field
  value: (return_path_value) @field.inner @entry.inside) @field.outer @entry.around

(content_type_field
  value: (_) @field.inner @entry.inside) @field.outer @entry.around

(content_transfer_encoding_field
  value: (encoding_mechanism) @field.inner @entry.inside) @field.outer @entry.around

(content_disposition_field
  value: (content_disposition) @field.inner @entry.inside) @field.outer @entry.around

(mime_version_field
  value: (_) @field.inner @entry.inside) @field.outer @entry.around

(content_description_field
  value: (unstructured) @field.inner @entry.inside) @field.outer @entry.around

(optional_field
  value: (unstructured) @field.inner @entry.inside) @field.outer @entry.around

(comment) @comment.outer @comment.around
(comment) @comment.inner @comment.inside

(quote_group) @quote.outer @comment.outer @comment.around

(quoted_block
  (quote_contents) @quote.inner @comment.inner @comment.inside)

(body
  (signature_separator)
  (signature) @signature.inner) @signature.outer

(mime_part
  (part_body) @mime_part.inner) @mime_part.outer

(body) @body.outer
(body_block) @body.inner
(epilogue)   @body.inner
