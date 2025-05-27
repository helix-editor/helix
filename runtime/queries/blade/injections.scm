((text) @injection.content
    (#set! injection.combined)
    (#set! injection.language "php"))

((comment) @injection.content
 (#set! injection.language "comment"))

((php_only) @injection.content
    (#set! injection.language "php-only"))

((parameter) @injection.content                                                                                                 
    (#set! injection.include-children)                                                                                          
    (#set! injection.language "php-only")) 

