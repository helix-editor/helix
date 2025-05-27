(term_declaration) @function.around

(type_declaration) @class.inside
(record) @class.inside

(comment) @comment.inside
(comment)+ @comment.around

(doc_block) @comment.around

(literal_list) @entry.around

(parenthesized_or_tuple_pattern) @entry.around

(pattern) @entry.around
