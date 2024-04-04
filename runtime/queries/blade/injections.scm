((text) @injection.content
    (#set! injection.combined)
    (#set! injection.language php))

((php_only) @injection.content
    (#set! injection.language php-only))
((parameter) @injection.content
    (#set! injection.language php-only))

