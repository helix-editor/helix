(keyword_definition) @indent
(test_case_definition) @indent

(for_statement) @indent
(for_statement "END" @outdent)
(for_statement
  right: (_ (arguments (continuation (ellipses) @outdent))))

(while_statement) @indent
(while_statement "END" @outdent)

(if_statement) @indent
(if_statement (elseif_statement) @outdent)
(if_statement (else_statement) @outdent)
(if_statement "END" @outdent)

(try_statement) @indent
(try_statement (except_statement) @outdent)
(try_statement (finally_statement) @outdent)
(try_statement (else_statement) @outdent)
(try_statement "END" @outdent)
