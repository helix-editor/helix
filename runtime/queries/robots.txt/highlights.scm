(comment) @comment

(directive) @attribute

(rule (unknownDirective) @attribute) @diagnostic.error

(userAgent) @namespace

(value) @string

((value) @string.special.path
  (#match? @string.special.path "^/"))

((value) @ui.text.directory
  (#match? @ui.text.directory "^/.+/$"))

((value) @operator
  (#match? @operator "\\*"))

(rule
  (directive (sitemap))
  (value) @string.special.url)

(rule
  (directive (crawlDelay))
  (value) @constant.numeric.integer)

":" @punctuation.delimiter
