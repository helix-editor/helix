(buildBlock . ("build") @keyword.function)
(taskBlock . ("task") @keyword.function )
(run . ("run") @keyword.function )
(taskBlock name: (identifier) @function )

(comment) @comment
(string) @string
(number) @constant.numeric
(identifier) @identifier

(include) @keyword.control.import
(let) @keyword.storage
(default) @keyword.storage
(config) @keyword.storage

(interpolation ["{" "}" "<" ">" ] @punctuation.special)
["{" "}" "<" ">" "(" ")" "[" "]"] @punctuation.bracket
["=>" "|"] @punctuation

; Statements
(build "build") @function
(config "config") @function
(copy "copy") @function
(default "default") @function
(delete "delete") @function
(depfile "depfile") @function
(envRemove "env-remove") @function
(from "from") @function
(info "info") @function
(let "let") @function
(setEnv "env") @function
(shell "shell") @function
(write "write") @function

; Expressions
(error "error" @function.builtin)
(getEnv "env" @function.builtin)
(glob "glob" @function.builtin)
(include "include" @function.builtin)
(info "info" @function.builtin)
(read "read" @function.builtin)
(warn "warn" @function.builtin)
(which "which" @function.builtin)

; Operations
(op (string)  @operator) 
("dedup" @operator)
("first" @operator)
("flatten" @operator)
("last" @operator)
("len" @operator)
("lines" @operator)
("tail" @operator)
(assertEq "assert-eq" @operator)
(discard "discard" @operator)
(filter "filter" @operator)
(filterMatch "filter-match" @operator)
(join "join" @operator)
(map "map" @operator)
(match "match" @operator)
(split "split" @operator)

