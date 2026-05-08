[(line_comment) (doc_comment)] @comment.inside

(line_comment)+ @comment.around
(doc_comment)+ @comment.around

(entry value: (_) @entry.inside) @entry.around
