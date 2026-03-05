; nvim-treesitter indentation heuristics for raw Perl inside EPL directives.

((code_tag (tag_content) @_perl) @indent.branch
  (#match? @_perl "^\\s*\\}\\s*(?:else|elsif|catch|continue|default|when)\\b.*\\{\\s*$"))

((line_code (line_content) @_perl) @indent.branch
  (#match? @_perl "^\\s*\\}\\s*(?:else|elsif|catch|continue|default|when)\\b.*\\{\\s*$"))

((code_tag (tag_content) @_perl) @indent.begin
  (#match? @_perl "\\{\\s*$"))

((line_code (line_content) @_perl) @indent.begin
  (#match? @_perl "\\{\\s*$"))

((line_code (line_content) @_perl) @indent.begin
  (#match? @_perl "begin\\s*$"))

((code_tag (tag_content) @_perl) @indent.end
  (#match? @_perl "^\\s*\\}"))

((line_code (line_content) @_perl) @indent.end
  (#match? @_perl "^\\s*\\}"))

((line_code (line_content) @_perl) @indent.end
  (#match? @_perl "^\\s*end"))
