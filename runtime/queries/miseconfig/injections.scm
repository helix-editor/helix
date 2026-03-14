; inherits: toml

; This part covers simple tasks where only the command(s) to run
; are specified as a string or array of strings, e.g.
;
;    [tasks]
;    simple = "simple-command arg1 arg2"
;    many-simple = [
;      "simple-command-1",
;      "simple-command-2",
;    ]
;
(table
  (bare_key) @table-name (#eq? @table-name "tasks")
  (pair (_) [
    ((string) @injection.shebang @injection.content (#set! injection.language "bash"))
    ((array (string) @injection.shebang @injection.content (#set! injection.language "bash")))
  ])
)

; This part covers advanced tasks which are specified as a table.
; Only the "run" key is subject to injections.
;
;    [tasks.foo]
;    description = "This is regular text."
;    run = "this is bash"
;
(table
  (dotted_key (bare_key) @table-name (#eq? @table-name "tasks"))
  (pair (bare_key) @key-name (#eq? @key-name "run") [
    ((string) @injection.shebang @injection.content (#set! injection.language "bash"))
    ((array (string) @injection.shebang @injection.content (#set! injection.language "bash")))
  ])
)
