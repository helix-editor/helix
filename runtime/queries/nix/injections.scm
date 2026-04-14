; Highlight links, TODOs, and similar markers inside comments by reusing the
; dedicated comment language.
((comment) @injection.content
 (#set! injection.language "comment"))

; Mark arbitrary languages via a preceding comment.
; Helix does not use upstream's #gsub! predicates here, so the comment text is
; passed through as-is.
((((comment) @injection.language) .
  (indented_string_expression (string_fragment) @injection.content))
 (#set! injection.combined))

; nixos testScript binding - value is Python.
((binding
   attrpath: (attrpath (identifier) @_path)
   expression: (indented_string_expression
     (string_fragment) @injection.content))
 (#eq? @_path "testScript")
 (#set! injection.language "python")
 (#set! injection.combined))

; nixos testScript binding with `let ... in ''...''` - value is Python.
((binding
   attrpath: (attrpath (identifier) @_path)
   expression: (let_expression
     body: (indented_string_expression
       (string_fragment) @injection.content)))
 (#eq? @_path "testScript")
 (#set! injection.language "python")
 (#set! injection.combined))

; Common binding-name -> bash injections.
; Covers Phase/Hook/Script conventions used across nixpkgs stdenv.
((binding
   attrpath: (attrpath (identifier) @_path)
   expression: [
     (indented_string_expression (string_fragment) @injection.content)
     (binary_expression (indented_string_expression (string_fragment) @injection.content))
   ])
 (#match? @_path "(^\\w*Phase|command|(pre|post)\\w*|(.*\\.)?\\w*([sS]cript|[hH]ook)|(.*\\.)?startup)$")
 (#set! injection.language "bash")
 (#set! injection.combined))

; builtins.{match,split} regex str
; Example: nix/tests/lang/eval-okay-regex-{match,split}.nix
((apply_expression
   function: (_) @_func
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_func "(^|\\.)match|split$")
 (#set! injection.language "regex")
 (#set! injection.combined))

; builtins.fromJSON json
; Example: nix/tests/lang/eval-okay-fromjson.nix
((apply_expression
   function: (_) @_func
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_func "(^|\\.)fromJSON$")
 (#set! injection.language "json")
 (#set! injection.combined))

; builtins.fromTOML toml
; Example: nix/tests/functional/lang/eval-okay-fromTOML.nix
((apply_expression
   function: (_) @_func
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_func "(^|\\.)fromTOML$")
 (#set! injection.language "toml")
 (#set! injection.combined))

; pkgs.writeShellScript / writeShellScriptBin - 2nd argument is bash.
((apply_expression
   function: (apply_expression function: (_) @_func)
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_func "(^|\\.)writeShellScript(Bin)?$")
 (#set! injection.language "bash")
 (#set! injection.combined))

; pkgs.runCommand variants - 3rd positional argument is bash.
(apply_expression
  (apply_expression
    function: (apply_expression
      function: ((_) @_func)))
    argument: (indented_string_expression (string_fragment) @injection.content)
  (#match? @_func "(^|\\.)runCommand(((No)?(CC))?(Local)?)?$")
  (#set! injection.language "bash")
  (#set! injection.combined))

; pkgs.writeShellApplication - the `text` attribute is bash.
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

; writeShellApplication with `text = let ... in "..."` - follow the let body.
(apply_expression
  function: ((_) @_func)
  argument: (_ (_)* (_ (_)* (binding
    attrpath: (attrpath (identifier) @_path)
    expression: (let_expression
      body: (indented_string_expression
        (string_fragment) @injection.content)))))
  (#match? @_func "(^|\\.)writeShellApplication$")
  (#match? @_path "^text$")
  (#set! injection.language "bash")
  (#set! injection.combined))

; lib.literalExpression / lib.literalExpressionPrefix - the string
; argument is a Nix expression shown in docs; highlight as nix.
; Uses specific node-type alternation rather than (_) to avoid
; interference with other query captures.
((apply_expression
   function: [
     (variable_expression (identifier) @_func)
     (select_expression attrpath: (attrpath attr: (identifier) @_func .))
   ]
   argument: (indented_string_expression
     (string_fragment) @injection.content))
 (#match? @_func "^literalExpression(Prefix)?$")
 (#set! injection.language "nix")
 (#set! injection.combined))

((apply_expression
   function: [
     (variable_expression (identifier) @_func)
     (select_expression attrpath: (attrpath attr: (identifier) @_func .))
   ]
   argument: (string_expression
     (string_fragment) @injection.content))
 (#match? @_func "^literalExpression(Prefix)?$")
 (#set! injection.language "nix"))

; pkgs.writeCBin name content
((apply_expression
   function: (apply_expression function: (_) @_func)
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_func "(^|\\.)writeC(Bin)?$")
 (#set! injection.language "c")
 (#set! injection.combined))

; pkgs.writers.write{Bash,Dash}[Bin] name content
((apply_expression
   function: (apply_expression function: (_) @_func)
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_func "(^|\\.)write[BD]ash(Bin)?$")
 (#set! injection.language "bash")
 (#set! injection.combined))

; pkgs.writers.writeFish[Bin] name content
((apply_expression
   function: (apply_expression function: (_) @_func)
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_func "(^|\\.)writeFish(Bin)?$")
 (#set! injection.language "fish")
 (#set! injection.combined))

; pkgs.writers.* usage examples: nixpkgs/pkgs/build-support/writers/test.nix

; pkgs.writers.writeRust[Bin] name attrs content
(apply_expression
  (apply_expression
    function: (apply_expression
      function: ((_) @_func)))
    argument: (indented_string_expression (string_fragment) @injection.content)
  (#match? @_func "(^|\\.)writeRust(Bin)?$")
  (#set! injection.language "rust")
  (#set! injection.combined))

; pkgs.writers.writeHaskell[Bin] name attrs content
(apply_expression
  (apply_expression
    function: (apply_expression
      function: ((_) @_func)))
    argument: (indented_string_expression (string_fragment) @injection.content)
  (#match? @_func "(^|\\.)writeHaskell(Bin)?$")
  (#set! injection.language "haskell")
  (#set! injection.combined))

; pkgs.writers.writeNim[Bin] name attrs content
(apply_expression
  (apply_expression
    function: (apply_expression
      function: ((_) @_func)))
    argument: (indented_string_expression (string_fragment) @injection.content)
  (#match? @_func "(^|\\.)writeNim(Bin)?$")
  (#set! injection.language "nim")
  (#set! injection.combined))

; pkgs.writers.writeJS[Bin] name attrs content
(apply_expression
  (apply_expression
    function: (apply_expression
      function: ((_) @_func)))
    argument: (indented_string_expression (string_fragment) @injection.content)
  (#match? @_func "(^|\\.)writeJS(Bin)?$")
  (#set! injection.language "javascript")
  (#set! injection.combined))

; pkgs.writers.writePerl[Bin] name attrs content
(apply_expression
  (apply_expression
    function: (apply_expression
      function: ((_) @_func)))
    argument: (indented_string_expression (string_fragment) @injection.content)
  (#match? @_func "(^|\\.)writePerl(Bin)?$")
  (#set! injection.language "perl")
  (#set! injection.combined))

; pkgs.writers.write{Python,PyPy}{2,3}[Bin] name attrs content
(apply_expression
  (apply_expression
    function: (apply_expression
      function: ((_) @_func)))
    argument: (indented_string_expression (string_fragment) @injection.content)
  (#match? @_func "(^|\\.)write(Python|PyPy)[23](Bin)?$")
  (#set! injection.language "python")
  (#set! injection.combined))

; pkgs.writers.writeNu[Bin] name attrs content
(apply_expression
  (apply_expression
    function: (apply_expression
      function: ((_) @_func)))
    argument: (indented_string_expression (string_fragment) @injection.content)
  (#match? @_func "(^|\\.)writeNu(Bin)?$")
  (#set! injection.language "nu")
  (#set! injection.combined))

; pkgs.writers.writeRuby[Bin] name attrs content
(apply_expression
  (apply_expression
    function: (apply_expression
      function: ((_) @_func)))
    argument: (indented_string_expression (string_fragment) @injection.content)
  (#match? @_func "(^|\\.)writeRuby(Bin)?$")
  (#set! injection.language "ruby")
  (#set! injection.combined))

; pkgs.writers.writeLua[Bin] name attrs content
(apply_expression
  (apply_expression
    function: (apply_expression
      function: ((_) @_func)))
    argument: (indented_string_expression (string_fragment) @injection.content)
  (#match? @_func "(^|\\.)writeLua(Bin)?$")
  (#set! injection.language "lua")
  (#set! injection.combined))

; pkgs.writers.writeNginxConfig name attrs content
(apply_expression
  (apply_expression
    function: (apply_expression
      function: ((_) @_func)))
    argument: (indented_string_expression (string_fragment) @injection.content)
  (#match? @_func "(^|\\.)writeNginxConfig$")
  (#set! injection.language "nginx")
  (#set! injection.combined))

; pkgs.writers.writeGuile[Bin] name attrs content
(apply_expression
  (apply_expression
    function: (apply_expression
      function: ((_) @_func)))
    argument: (indented_string_expression (string_fragment) @injection.content)
  (#match? @_func "(^|\\.)writeGuile(Bin)?$")
  (#set! injection.language "scheme")
  (#set! injection.combined))

; pkgs.writers.writeBabashka[Bin] name attrs content
(apply_expression
  (apply_expression
    function: (apply_expression
      function: ((_) @_func)))
    argument: (indented_string_expression (string_fragment) @injection.content)
  (#match? @_func "(^|\\.)writeBabashka(Bin)?$")
  (#set! injection.language "clojure")
  (#set! injection.combined))

; Filename-based injection for indented strings.
;
; Detect the language from the file extension of a preceding filename
; argument in a curried call:
;
;   pkgs.writeText "index.html" ''
;     <div>Hello</div>
;   ''
;   pkgs.writeShellScriptBin "run.sh" ''
;     echo hi
;   ''
;
; The pattern matches `f "name.ext" '' ... ''` for any function `f`,
; minus a small denylist of common nixpkgs idioms that take a
; filename-shaped string but are not file writers (`removeSuffix`,
; `trace`, `throw`, etc.). Outside the denylist, false positives are
; tolerated: the worst case is mis-highlighting, whereas a false
; negative means no highlighting at all.
;
; Concept harvested from nix-community/tree-sitter-nix#169 by
; @nuketownada; rewritten as a hand-maintained list rather than
; generated from a Nix derivation, with the function denylist added
; per adversarial review of #53.
((apply_expression
   function: (apply_expression
     function: (_) @_inner_func
     argument: (string_expression (string_fragment) @_filename))
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_filename "\\.(sh|bash)$")
 (#not-match? @_inner_func "(^|\\.)(removeSuffix|hasSuffix|hasPrefix|removePrefix|trace|throw|warn|warnIf|abort|assertMsg|seq|deepSeq|writeShellScript|writeShellScriptBin|runCommand|runCommandLocal|runCommandCC|runCommandNoCC)$")
 (#set! injection.language "bash")
 (#set! injection.combined))

((apply_expression
   function: (apply_expression
     function: (_) @_inner_func
     argument: (string_expression (string_fragment) @_filename))
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_filename "\\.(py)$")
 (#not-match? @_inner_func "(^|\\.)(removeSuffix|hasSuffix|hasPrefix|removePrefix|trace|throw|warn|warnIf|abort|assertMsg|seq|deepSeq|writeShellScript|writeShellScriptBin|runCommand|runCommandLocal|runCommandCC|runCommandNoCC)$")
 (#set! injection.language "python")
 (#set! injection.combined))

((apply_expression
   function: (apply_expression
     function: (_) @_inner_func
     argument: (string_expression (string_fragment) @_filename))
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_filename "\\.(html|htm)$")
 (#not-match? @_inner_func "(^|\\.)(removeSuffix|hasSuffix|hasPrefix|removePrefix|trace|throw|warn|warnIf|abort|assertMsg|seq|deepSeq|writeShellScript|writeShellScriptBin|runCommand|runCommandLocal|runCommandCC|runCommandNoCC)$")
 (#set! injection.language "html")
 (#set! injection.combined))

((apply_expression
   function: (apply_expression
     function: (_) @_inner_func
     argument: (string_expression (string_fragment) @_filename))
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_filename "\\.(css)$")
 (#not-match? @_inner_func "(^|\\.)(removeSuffix|hasSuffix|hasPrefix|removePrefix|trace|throw|warn|warnIf|abort|assertMsg|seq|deepSeq|writeShellScript|writeShellScriptBin|runCommand|runCommandLocal|runCommandCC|runCommandNoCC)$")
 (#set! injection.language "css")
 (#set! injection.combined))

((apply_expression
   function: (apply_expression
     function: (_) @_inner_func
     argument: (string_expression (string_fragment) @_filename))
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_filename "\\.(js|mjs|cjs)$")
 (#not-match? @_inner_func "(^|\\.)(removeSuffix|hasSuffix|hasPrefix|removePrefix|trace|throw|warn|warnIf|abort|assertMsg|seq|deepSeq|writeShellScript|writeShellScriptBin|runCommand|runCommandLocal|runCommandCC|runCommandNoCC)$")
 (#set! injection.language "javascript")
 (#set! injection.combined))

((apply_expression
   function: (apply_expression
     function: (_) @_inner_func
     argument: (string_expression (string_fragment) @_filename))
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_filename "\\.(ts|mts|cts)$")
 (#not-match? @_inner_func "(^|\\.)(removeSuffix|hasSuffix|hasPrefix|removePrefix|trace|throw|warn|warnIf|abort|assertMsg|seq|deepSeq|writeShellScript|writeShellScriptBin|runCommand|runCommandLocal|runCommandCC|runCommandNoCC)$")
 (#set! injection.language "typescript")
 (#set! injection.combined))

((apply_expression
   function: (apply_expression
     function: (_) @_inner_func
     argument: (string_expression (string_fragment) @_filename))
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_filename "\\.(json)$")
 (#not-match? @_inner_func "(^|\\.)(removeSuffix|hasSuffix|hasPrefix|removePrefix|trace|throw|warn|warnIf|abort|assertMsg|seq|deepSeq|writeShellScript|writeShellScriptBin|runCommand|runCommandLocal|runCommandCC|runCommandNoCC)$")
 (#set! injection.language "json")
 (#set! injection.combined))

((apply_expression
   function: (apply_expression
     function: (_) @_inner_func
     argument: (string_expression (string_fragment) @_filename))
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_filename "\\.(yml|yaml)$")
 (#not-match? @_inner_func "(^|\\.)(removeSuffix|hasSuffix|hasPrefix|removePrefix|trace|throw|warn|warnIf|abort|assertMsg|seq|deepSeq|writeShellScript|writeShellScriptBin|runCommand|runCommandLocal|runCommandCC|runCommandNoCC)$")
 (#set! injection.language "yaml")
 (#set! injection.combined))

((apply_expression
   function: (apply_expression
     function: (_) @_inner_func
     argument: (string_expression (string_fragment) @_filename))
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_filename "\\.(toml)$")
 (#not-match? @_inner_func "(^|\\.)(removeSuffix|hasSuffix|hasPrefix|removePrefix|trace|throw|warn|warnIf|abort|assertMsg|seq|deepSeq|writeShellScript|writeShellScriptBin|runCommand|runCommandLocal|runCommandCC|runCommandNoCC)$")
 (#set! injection.language "toml")
 (#set! injection.combined))

((apply_expression
   function: (apply_expression
     function: (_) @_inner_func
     argument: (string_expression (string_fragment) @_filename))
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_filename "\\.(lua)$")
 (#not-match? @_inner_func "(^|\\.)(removeSuffix|hasSuffix|hasPrefix|removePrefix|trace|throw|warn|warnIf|abort|assertMsg|seq|deepSeq|writeShellScript|writeShellScriptBin|runCommand|runCommandLocal|runCommandCC|runCommandNoCC)$")
 (#set! injection.language "lua")
 (#set! injection.combined))

((apply_expression
   function: (apply_expression
     function: (_) @_inner_func
     argument: (string_expression (string_fragment) @_filename))
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_filename "\\.(nix)$")
 (#not-match? @_inner_func "(^|\\.)(removeSuffix|hasSuffix|hasPrefix|removePrefix|trace|throw|warn|warnIf|abort|assertMsg|seq|deepSeq|writeShellScript|writeShellScriptBin|runCommand|runCommandLocal|runCommandCC|runCommandNoCC)$")
 (#set! injection.language "nix")
 (#set! injection.combined))

((apply_expression
   function: (apply_expression
     function: (_) @_inner_func
     argument: (string_expression (string_fragment) @_filename))
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_filename "\\.(xml)$")
 (#not-match? @_inner_func "(^|\\.)(removeSuffix|hasSuffix|hasPrefix|removePrefix|trace|throw|warn|warnIf|abort|assertMsg|seq|deepSeq|writeShellScript|writeShellScriptBin|runCommand|runCommandLocal|runCommandCC|runCommandNoCC)$")
 (#set! injection.language "xml")
 (#set! injection.combined))

((apply_expression
   function: (apply_expression
     function: (_) @_inner_func
     argument: (string_expression (string_fragment) @_filename))
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_filename "\\.(md)$")
 (#not-match? @_inner_func "(^|\\.)(removeSuffix|hasSuffix|hasPrefix|removePrefix|trace|throw|warn|warnIf|abort|assertMsg|seq|deepSeq|writeShellScript|writeShellScriptBin|runCommand|runCommandLocal|runCommandCC|runCommandNoCC)$")
 (#set! injection.language "markdown")
 (#set! injection.combined))

((apply_expression
   function: (apply_expression
     function: (_) @_inner_func
     argument: (string_expression (string_fragment) @_filename))
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_filename "\\.(sql)$")
 (#not-match? @_inner_func "(^|\\.)(removeSuffix|hasSuffix|hasPrefix|removePrefix|trace|throw|warn|warnIf|abort|assertMsg|seq|deepSeq|writeShellScript|writeShellScriptBin|runCommand|runCommandLocal|runCommandCC|runCommandNoCC)$")
 (#set! injection.language "sql")
 (#set! injection.combined))

((apply_expression
   function: (apply_expression function: (_) @_func
     argument: (string_expression (string_fragment) @injection.filename))
   argument: (indented_string_expression (string_fragment) @injection.content))
 (#match? @_func "(^|\\.)write(Text|Script(Bin)?)$")
 (#set! injection.combined))

; Let Helix infer a language from a shebang at the start of an indented string.
((indented_string_expression (string_fragment) @injection.shebang @injection.content)
 (#set! injection.combined))
