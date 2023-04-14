(block_scalar) @indent @extend

((block_mapping_pair
    key: (_) @key
    value: (_)? @val
    (#not-same-line? @key @val)
  ) @indent @extend
)
