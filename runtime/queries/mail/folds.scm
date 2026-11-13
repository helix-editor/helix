; Fold the entire header block.
(header) @fold

; Fold long recipient lists and individual Received: trace headers.
(address_list) @fold
(received_field) @fold

; Fold MIME sections independently.
(mime_part) @fold
(part_headers) @fold
(part_body) @fold
(multipart_body) @fold

; Fold body paragraphs, quote groups, and signatures.
(body_block) @fold
(quote_group) @fold
(quoted_block) @fold
(signature) @fold
