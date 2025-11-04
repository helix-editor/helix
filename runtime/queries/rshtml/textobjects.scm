(raw_block (html_text)? @entry.inside) @entry.around

(section_block body: (_)? @entry.inside) @entry.around

(rust_block content: (rust_text)? @entry.inside) @entry.around


(if_stmt body: (_)? @entry.inside) @entry.around

(while_stmt body: (_)? @entry.inside) @entry.around

(for_stmt body: (_)? @entry.inside) @entry.around

(match_stmt (match_stmt_arm) @entry.inside) 
(match_stmt (match_stmt_arm)+ @entry.inside) @entry.around

(component_tag (component_tag_parameter (rust_identifier) @parameter.inside) @parameter.around)
(component_tag body: (component_tag_body)? @entry.inside) @entry.around

(comment_block (comment_content) @comment.inside) @comment.around
