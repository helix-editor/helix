; inherits: yaml

((document (block_node (block_scalar) @injection.content))
 (#set! injection.language "llvm"))

((document (block_node (block_mapping (block_mapping_pair
  key: (flow_node (plain_scalar (string_scalar))) ; "body"
  value: (block_node (block_scalar) @injection.content)))))
  (#set! injection.language "mir"))
