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

; Handle partially typed code.
(ERROR [(function_command) (macro_command) (if_command) (foreach_command) (while_command)] @indent @extend)
