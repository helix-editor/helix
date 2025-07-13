(atom_block
  (atom) @entry.inside) @entry.around

(email_address) @entry.around
(header_other
  (header_unstructured) @entry.around)

(quoted_block)+ @comment.around

(body_block)+ @function.around

(header_subject
  (subject) @function.around)
