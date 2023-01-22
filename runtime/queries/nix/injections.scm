((comment) @injection.content
 (#set! injection.language "comment"))

; mark arbitary languages with a comment
((((comment) @injection.language) .
  (indented_string_expression (string_fragment) @injection.content))
  (#set! injection.combined))

; Common attribute keys corresponding to scripts,
; such as those of stdenv.mkDerivation.
((binding
   attrpath: (attrpath (identifier) @_path)
   expression: (indented_string_expression
     (string_fragment) @injection.content))
 (#match? @_path "(^\\w*Phase|(pre|post)\\w*|(.*\\.)?\\w*([sS]cript|[hH]ook)|(.*\\.)?startup)$")
 (#set! injection.language "bash")
 (#set! injection.combined))

; trivial-builders.nix pkgs.writeShellScript[Bin] name content
((apply_expression
   function: (apply_expression function: (_) @_func)
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_func "(^|\\.)writeShellScript(Bin)?$")
 (#set! injection.language "bash")
 (#set! injection.combined))

; trivial-builders.nix, aliases.nix
; pkgs.runCommand[[No]CC][Local] name attrs content
(apply_expression
  (apply_expression
    function: (apply_expression
      function: ((_) @_func)))
    argument: (indented_string_expression (string_fragment) @injection.content)
  (#match? @_func "(^|\\.)runCommand(((No)?(CC))?(Local)?)?$")
  (#set! injection.language "bash")
  (#set! injection.combined))

; trivial-builders.nix pkgs.writeShellApplication { text = content; }
(apply_expression
  function: ((_) @_func)
  argument: (_ (_)* (_ (_)* (binding
    attrpath: (attrpath (identifier) @_path)
     expression: (indented_string_expression
       (string_fragment) @injection.content))))
  (#match? @_func "(^|\\.)writeShellApplication$")
  (#match? @_path "^text$")
  (#set! injection.language "bash")
  (#set! injection.combined))

; trivial-builders.nix pkgs.writeCBin name content
((apply_expression
   function: (apply_expression function: (_) @_func)
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_func "(^|\\.)writeCBin$")
 (#set! injection.language "c")
 (#set! injection.combined))
