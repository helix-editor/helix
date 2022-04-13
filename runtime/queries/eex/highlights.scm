; https://github.com/connorlay/tree-sitter-eex/blob/f742f2fe327463335e8671a87c0b9b396905d1d1/queries/highlights.scm

; wrapping in (directive .. ) prevents us from highlighting '%>' in a comment as a keyword
(directive ["<%" "<%=" "<%%" "<%%=" "%>"] @keyword)

(comment) @comment
