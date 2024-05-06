(header
  (headertype) @keyword
)

[
  (from)
  (to)
  (cc)
  (bcc)
  (sender)
  (replyto)
  (message_id)
  (in_reply_to)
  (references)
  (date)
  (subject)
] @keyword

[
  (year) 
  (day)
  (hour)
  (minute)
  (second)
  (zone)
] @constant.numeric

[
  (month) 
  (day_name)
] @tag

(subjectheader
  (subject)
  (_) @string)

(fws) @string

[
  (domain)
  (addrspec)
] @string.special.url

[
  (quote1)
  (quote2)
] @markup.quote

[ 
  ":"
  ","
  ";"
] @punctuation.delimiter

[
  "<"
  ">"
  "("
  ")"
  "["
  "]"
] @punctuation.bracket
