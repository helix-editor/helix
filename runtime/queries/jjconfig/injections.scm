((comment) @injection.content
 (#set! injection.language "comment"))

(table
 (bare_key) @table-name (#any-of? @table-name "templates" "template-aliases")
 [(pair (_) ((string) @injection.content (#set! injection.language "jjtemplate"))) (comment)])

(table
 (bare_key) @table-name (#any-of? @table-name "revsets" "revset-aliases")
 [(pair (_) ((string) @injection.content (#set! injection.language "jjrevset"))) (comment)])
