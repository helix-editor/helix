[
  (if_condition)
  (foreach_loop)
  (while_loop)
  (function_def)
  (macro_def)
  (block_def)
  (normal_command)
] @indent

")" @outdent

[
  (else)
  (elseif)
  (endif)
  (endforeach)
  (endwhile)
  (endfunction)
  (endmacro)
  (endblock)
] @outdent
