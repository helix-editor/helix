; select jobs
(block_mapping
  (block_mapping_pair
    value: (block_node
             (block_mapping
               (block_mapping_pair
                 key: (flow_node) @_key (#eq? @_key "stage"))))) @definition.struct)

; select defined variables under `variables:`
(block_mapping
  (block_mapping_pair
    key: (flow_node) @_key (#eq? @_key "variables")
    value: (block_node
             (block_mapping
               (block_mapping_pair
                 key: (flow_node) @name
                 value: (_) @definition.constant)))))
