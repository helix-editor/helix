([(comment) (block_comment)] @injection.content
 (#set! injection.language "comment"))


; Matches these SQL interpolators:
;  - Doobie: 'sql', 'fr'
;  - Quill: 'sql', 'infix'
;  - Slick: 'sql', 'sqlu'
(interpolated_string_expression 
  interpolator: 
    ((identifier) @interpolator 
     (#any-of? @interpolator "fr" "infix" "sql" "sqlu"))
  (interpolated_string) @injection.content
  (#set! injection.language "sql"))

