(comment) @comment

(shebang_recipe 
    (shebang 
        interpreter:(TEXT) @language)
    (shebang_body) @content
) 

(source_file 
    (item (setting lang:(NAME) @language))
    (item (recipe (body (recipe_body) @content)))
) 

; (interpolation (expression) @just)
