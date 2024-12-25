
(comment) @comment

(command name: (simple_word) @function)

"proc" @keyword.function

(procedure
  name: (_) @variable
)

(set (simple_word) @variable)

(argument
  name: (_) @variable.parameter
)

((simple_word) @variable.builtin
               (#any-of? @variable.builtin
                "argc"
                "argv"
                "argv0"
                "auto_path"
                "env"
                "errorCode"
                "errorInfo"
                "tcl_interactive"
                "tcl_library"
                "tcl_nonwordchars"
                "tcl_patchLevel"
                "tcl_pkgPath"
                "tcl_platform"
                "tcl_precision"
                "tcl_rcFileName"
                "tcl_traceCompile"
                "tcl_traceExec"
                "tcl_wordchars"
                "tcl_version"))


"expr" @function.builtin

(command
  name: (simple_word) @function.builtin
  (#any-of? @function.builtin
   "cd"
   "exec"
   "exit"
   "incr"
   "info"
   "join"
   "puts"
   "regexp"
   "regsub"
   "split"
   "subst"
   "trace"
   "source"))

(command name: (simple_word) @keyword
         (#any-of? @keyword
          "append"
          "break"
          "catch"
          "continue"
          "default"
          "dict"
          "error"
          "eval"
          "global"
          "lappend"
          "lassign"
          "lindex"
          "linsert"
          "list"
          "llength"
          "lmap"
          "lrange"
          "lrepeat"
          "lreplace"
          "lreverse"
          "lsearch"
          "lset"
          "lsort"
          "package"
          "return"
          "switch"
          "throw"
          "unset"
          "variable"))

[
 "error"
 "namespace"
 "on"
 "set"
 "try"
 ] @keyword

(unpack) @operator

[
 "while"
 "foreach"
 ; "for"
 ] @keyword.control.repeat

[
 "if"
 "else"
 "elseif"
 ] @keyword.control.conditional

[
 "**"
 "/" "*" "%" "+" "-"
 "<<" ">>"
 ">" "<" ">=" "<="
 "==" "!="
 "eq" "ne"
 "in" "ni"
 "&"
 "^"
 "|"
 "&&"
 "||"
 ] @operator

(variable_substitution) @variable
(quoted_word) @string
(escaped_character) @constant.character.escape

[
 "{" "}"
 "[" "]"
 ";"
 ] @punctuation.delimiter

((simple_word) @constant.numeric
               (#match? @constant.numeric "^[0-9]+$"))

((simple_word) @constant.builtin.boolean
               (#any-of? @constant.builtin.boolean "true" "false"))

; after apply array auto_execok auto_import auto_load auto_mkindex auto_qualify
; auto_reset bgerror binary chan clock close coroutine dde encoding eof fblocked
; fconfigure fcopy file fileevent filename flush format gets glob history http
; interp load mathfunc mathop memory msgcat my next nextto open parray pid
; pkg::create pkg_mkIndex platform platform::shell pwd re_syntax read refchan
; registry rename safe scan seek self socket source string tailcall tcl::prefix
; tcl_endOfWord tcl_findLibrary tcl_startOfNextWord tcl_startOfPreviousWord
; tcl_wordBreakAfter tcl_wordBreakBefore tcltest tell time timerate tm
; transchan unknown unload update uplevel upvar vwait yield yieldto zlib
