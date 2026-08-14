(comment) @comment.line

[
  (path_pattern)
  (quoted_path)
] @string.special.path

(quoted_argument) @string
(escape_sequence) @constant.character.escape

(integer) @constant.numeric.integer
(size) @constant.numeric

((directive
  name: (directive_name) @keyword.directive)
  (#match? @keyword.directive "^(addextension|allowhardlink|compress|compresscmd|compressext|compressoptions|copy|copytruncate|create|createolddir|daily|dateext|dateformat|datehourago|dateyesterday|delaycompress|errors|extension|firstaction|hourly|ifempty|ignoreduplicates|lastaction|mail|mailfirst|maillast|maxage|maxsize|minage|minsize|minutes|missingok|monthly|noallowhardlink|nocompress|nocopy|nocopytruncate|nocreate|nocreateolddir|nodateext|nodatehourago|nodateyesterday|nodelaycompress|nomail|nomissingok|noolddir|norenamecopy|nosharedscripts|noshred|notifempty|olddir|postrotate|preremove|prerotate|renamecopy|rotate|sharedscripts|shred|shredcycles|size|start|su|tabooext|taboopat|uncompresscmd|weekly|yearly)$"))

((directive
  name: (directive_name) @variable.other.member)
  (#not-match? @variable.other.member "^(addextension|allowhardlink|compress|compresscmd|compressext|compressoptions|copy|copytruncate|create|createolddir|daily|dateext|dateformat|datehourago|dateyesterday|delaycompress|errors|extension|firstaction|hourly|ifempty|ignoreduplicates|lastaction|mail|mailfirst|maillast|maxage|maxsize|minage|minsize|minutes|missingok|monthly|noallowhardlink|nocompress|nocopy|nocopytruncate|nocreate|nocreateolddir|nodateext|nodatehourago|nodateyesterday|nodelaycompress|nomail|nomissingok|noolddir|norenamecopy|nosharedscripts|noshred|notifempty|olddir|postrotate|preremove|prerotate|renamecopy|rotate|sharedscripts|shred|shredcycles|size|start|su|tabooext|taboopat|uncompresscmd|weekly|yearly)$"))

(include_directive
  name: (directive_name) @keyword.control.import)

[
  (script_directive)
  (endscript)
] @keyword.directive

"=" @operator

[
  "{"
  "}"
] @punctuation.bracket
