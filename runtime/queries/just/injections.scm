((comment) @injection.content
 (#set! injection.language "comment"))

(shebang_recipe
 (shebang interpreter:(TEXT) @injection.language)
 (shebang_body) @injection.content
 (#set! injection.include-children))

(source_file
 (item (setting lang:(NAME) @injection.language))
 (item (recipe (body (recipe_body) @injection.content)))
 (#set! injection.include-children))

; (interpolation (expression) @just)