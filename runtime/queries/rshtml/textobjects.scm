(rust_block content: (rust_text)? @entry.inside) @entry.around


(if_stmt body: (_)? @entry.inside) @entry.around

(while_stmt body: (_)? @entry.inside) @entry.around

(for_stmt body: (_)? @entry.inside) @entry.around

(component_tag (component_tag_parameter (rust_identifier) @parameter.inside) @parameter.around)
(component_tag body: (component_tag_body)? @entry.inside) @entry.around
