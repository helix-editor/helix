; Comments
([
  (comment_tag)
  (line_comment)
] @comment)

; Embedded Perl content
[
  (tag_content)
  (line_content)
] @embedded

; Tag and line directive delimiters
(code_tag "<%" @keyword.directive)
(expression_tag "<%=" @keyword.directive)
(raw_expression_tag "<%==" @keyword.directive)
(comment_tag "<%#" @keyword.directive)

(line_code "%" @keyword.directive)
(line_expression "%=" @keyword.directive)
(line_raw_expression "%==" @keyword.directive)
(line_comment "%#" @keyword.directive)
(line_escaped_percent "%%" @escape)

(escaped_open_tag) @escape

(tag_close "%>" @keyword.directive)
(tag_close "=" @keyword.directive "%>" @keyword.directive)
