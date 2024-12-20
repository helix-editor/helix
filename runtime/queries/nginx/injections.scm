; Script tags
((lua_code) @injection.content
 (#set! injection.language "lua")
 (#set! injection.combined))

((regex) @injection.content
 (#set! injection.language "regex"))
