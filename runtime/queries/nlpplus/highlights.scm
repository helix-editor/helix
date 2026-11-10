; Patterns don't overlap (the predicates keep them apart), so the result is
; the same whichever priority order an editor uses.
(comment) @comment

; ---- Regions ----------------------------------------------------------------
(section_start) @keyword.directive

(section_end) @keyword.directive

(recurse_section
  name: (identifier) @label)

(recurse_section
  end_name: (identifier) @label)

; ---- Rules ------------------------------------------------------------------
"<-" @operator

"@@" @punctuation.special

; Node constants match classes of text rather than a named node.
((node_name) @constant.builtin
  (#match? @constant.builtin "^_(x[A-Z]+|ROOT)$"))

((node_name) @type
  (#not-match? @type "^_(x[A-Z]+|ROOT)$"))

; Literal words, characters and symbols matched in the text
(rule_element
  name: (identifier) @string.special)

(rule_element
  name: (symbol) @string.special)

(rule_element
  name: (number) @constant.numeric.integer)

(escape_char) @constant.character.escape

(modifier
  name: (identifier) @attribute)

(modifier
  value: (identifier) @constant)

; Words in a match=(...) list are literal text, like rule literals.
(modifier_list
  (identifier) @string.special)

(modifier_list
  (symbol) @string.special)

; ---- Code -------------------------------------------------------------------
"if" @keyword.control.conditional

"else" @keyword.control.conditional

"while" @keyword.control.repeat

"return" @keyword.control.return

; G("x"), L("x"), N("x"), S("x"), X("x"), P("x")
(variable_scope) @variable.builtin

(function_definition
  name: (identifier) @function)

(call_expression
  scope: (identifier) @namespace)

((call_expression
  function: (identifier) @function.builtin)
  (#any-of? @function.builtin
    "LJ" "abs" "addarg" "addattr" "addcnode" "addconcept" "addconval" "addnode" "addnumval"
    "addstmt" "addstrs" "addstrval" "addsval" "addword" "arraylength" "attrchange" "attrexists"
    "attrname" "attrtype" "attrvals" "attrwithval" "batchstart" "cap" "cbuf" "ceiling" "closefile"
    "conceptname" "conceptpath" "conval" "cout" "coutreset" "dballocstmt" "dbbindcol" "dbclose"
    "dbexec" "dbexecstmt" "dbfetch" "dbfreestmt" "dbopen" "deaccent" "debug" "dictfindword"
    "dictfirst" "dictgetword" "dictnext" "down" "eltnode" "excise" "exitpass" "exittopopup"
    "factorial" "fail" "fileout" "findana" "findattr" "findattrs" "findconcept" "findhierconcept"
    "findnode" "findphrase" "findroot" "findvals" "findwordpath" "firstnode" "floor" "flt" "fltval"
    "fncallstart" "fprintgvar" "fprintnvar" "fprintvar" "fprintxvar" "gdump" "getconcept"
    "getconval" "getnumval" "getpopupdata" "getstrval" "getsval" "ginc" "gp" "group" "gtolower"
    "guniq" "hitconf" "inc" "inheritval" "inputrange" "inputrangetofile" "interactive" "kbdumptree"
    "lasteltnode" "lastnode" "length" "lengthr" "levenshtein" "lextagger" "listadd" "listnode" "lj"
    "log" "logten" "lookup" "lowercase" "makeconcept" "makeparentconcept" "makephrase" "makestmt"
    "merge" "merger" "mkdir" "mod" "movecleft" "movecright" "movesem" "ndump" "next" "nextattr"
    "nextval" "ninc" "nodeconcept" "nodeowner" "noop" "num" "numrange" "numval" "openfile"
    "pathconcept" "percentstr" "permuten" "phraselength" "phraseraw" "phrasetext" "pncopyvars"
    "pndeletechilds" "pndown" "pninsert" "pnmakevar" "pnname" "pnnext" "pnprev" "pnrename"
    "pnreplaceval" "pnroot" "pnsingletdown" "pnup" "pnvar" "pnvarnames" "pow" "pranchor" "prchild"
    "preaction" "prev" "print" "printr" "printvar" "prlit" "prrange" "prtree" "prunephrases"
    "prxtree" "randomint" "regexp" "regexpi" "renameattr" "renamechild" "renameconcept" "renamenode"
    "replaceval" "resolveurl" "returnstmt" "rfaaction" "rfaactions" "rfaarg" "rfaargtolist"
    "rfacode" "rfaelement" "rfaelt" "rfaexpr" "rfalist" "rfalitelt" "rfalittoaction" "rfalittopair"
    "rfaname" "rfanodes" "rfanonlit" "rfanonlitelt" "rfanum" "rfaop" "rfapair" "rfapairs"
    "rfapostunary" "rfapres" "rfarange" "rfarecurse" "rfarecurses" "rfaregion" "rfaregions"
    "rfarule" "rfarulelts" "rfarulemark" "rfarules" "rfarulesfile" "rfaselect" "rfastr" "rfasugg"
    "rfaunary" "rfavar" "rfbarg" "rfbdecl" "rfbdecls" "rightjustifynum" "rmattr" "rmattrs"
    "rmattrval" "rmchild" "rmchildren" "rmconcept" "rmcphrase" "rmnode" "rmphrase" "rmval" "rmvals"
    "rmword" "round" "sdump" "setbase" "setlookahead" "setunsealed" "single" "singler" "singlex"
    "singlezap" "sortchilds" "sortconsbyattr" "sorthier" "sortphrase" "sortvals" "spellcandidates"
    "spellcorrect" "spellword" "splice" "split" "sqlstr" "sqrt" "startout" "stem" "stopout" "str"
    "strchar" "strchr" "strchrcount" "strclean" "strcontains" "strcontainsnocase" "strendswith"
    "strequal" "strequalnocase" "strescape" "strgreaterthan" "strisalpha" "strisdigit" "strislower"
    "strisupper" "strlength" "strlessthan" "strnotequal" "strnotequalnocase" "strpiece" "strrchr"
    "strspellcandidate" "strspellcompare" "strstartswith" "strsubst" "strtolower" "strtotitle"
    "strtoupper" "strtrim" "strunescape" "strval" "strwrap" "succeed" "suffix" "system" "take"
    "today" "topdir" "truncate" "unknown" "unpackdirs" "up" "uppercase" "urlbase" "urltofile" "var"
    "vareq" "varfn" "varfnarray" "varinlist" "varne" "varstrs" "varz" "whilestmt"
    "wnhypnymstoconcept" "wninit" "wnsensestoconcept" "wordindex" "wordpath" "writekb" "xaddlen"
    "xaddnvar" "xdump" "xinc" "xmlstr" "xrename"))

((call_expression
  function: (identifier) @function)
  (#not-any-of? @function
    "LJ" "abs" "addarg" "addattr" "addcnode" "addconcept" "addconval" "addnode" "addnumval"
    "addstmt" "addstrs" "addstrval" "addsval" "addword" "arraylength" "attrchange" "attrexists"
    "attrname" "attrtype" "attrvals" "attrwithval" "batchstart" "cap" "cbuf" "ceiling" "closefile"
    "conceptname" "conceptpath" "conval" "cout" "coutreset" "dballocstmt" "dbbindcol" "dbclose"
    "dbexec" "dbexecstmt" "dbfetch" "dbfreestmt" "dbopen" "deaccent" "debug" "dictfindword"
    "dictfirst" "dictgetword" "dictnext" "down" "eltnode" "excise" "exitpass" "exittopopup"
    "factorial" "fail" "fileout" "findana" "findattr" "findattrs" "findconcept" "findhierconcept"
    "findnode" "findphrase" "findroot" "findvals" "findwordpath" "firstnode" "floor" "flt" "fltval"
    "fncallstart" "fprintgvar" "fprintnvar" "fprintvar" "fprintxvar" "gdump" "getconcept"
    "getconval" "getnumval" "getpopupdata" "getstrval" "getsval" "ginc" "gp" "group" "gtolower"
    "guniq" "hitconf" "inc" "inheritval" "inputrange" "inputrangetofile" "interactive" "kbdumptree"
    "lasteltnode" "lastnode" "length" "lengthr" "levenshtein" "lextagger" "listadd" "listnode" "lj"
    "log" "logten" "lookup" "lowercase" "makeconcept" "makeparentconcept" "makephrase" "makestmt"
    "merge" "merger" "mkdir" "mod" "movecleft" "movecright" "movesem" "ndump" "next" "nextattr"
    "nextval" "ninc" "nodeconcept" "nodeowner" "noop" "num" "numrange" "numval" "openfile"
    "pathconcept" "percentstr" "permuten" "phraselength" "phraseraw" "phrasetext" "pncopyvars"
    "pndeletechilds" "pndown" "pninsert" "pnmakevar" "pnname" "pnnext" "pnprev" "pnrename"
    "pnreplaceval" "pnroot" "pnsingletdown" "pnup" "pnvar" "pnvarnames" "pow" "pranchor" "prchild"
    "preaction" "prev" "print" "printr" "printvar" "prlit" "prrange" "prtree" "prunephrases"
    "prxtree" "randomint" "regexp" "regexpi" "renameattr" "renamechild" "renameconcept" "renamenode"
    "replaceval" "resolveurl" "returnstmt" "rfaaction" "rfaactions" "rfaarg" "rfaargtolist"
    "rfacode" "rfaelement" "rfaelt" "rfaexpr" "rfalist" "rfalitelt" "rfalittoaction" "rfalittopair"
    "rfaname" "rfanodes" "rfanonlit" "rfanonlitelt" "rfanum" "rfaop" "rfapair" "rfapairs"
    "rfapostunary" "rfapres" "rfarange" "rfarecurse" "rfarecurses" "rfaregion" "rfaregions"
    "rfarule" "rfarulelts" "rfarulemark" "rfarules" "rfarulesfile" "rfaselect" "rfastr" "rfasugg"
    "rfaunary" "rfavar" "rfbarg" "rfbdecl" "rfbdecls" "rightjustifynum" "rmattr" "rmattrs"
    "rmattrval" "rmchild" "rmchildren" "rmconcept" "rmcphrase" "rmnode" "rmphrase" "rmval" "rmvals"
    "rmword" "round" "sdump" "setbase" "setlookahead" "setunsealed" "single" "singler" "singlex"
    "singlezap" "sortchilds" "sortconsbyattr" "sorthier" "sortphrase" "sortvals" "spellcandidates"
    "spellcorrect" "spellword" "splice" "split" "sqlstr" "sqrt" "startout" "stem" "stopout" "str"
    "strchar" "strchr" "strchrcount" "strclean" "strcontains" "strcontainsnocase" "strendswith"
    "strequal" "strequalnocase" "strescape" "strgreaterthan" "strisalpha" "strisdigit" "strislower"
    "strisupper" "strlength" "strlessthan" "strnotequal" "strnotequalnocase" "strpiece" "strrchr"
    "strspellcandidate" "strspellcompare" "strstartswith" "strsubst" "strtolower" "strtotitle"
    "strtoupper" "strtrim" "strunescape" "strval" "strwrap" "succeed" "suffix" "system" "take"
    "today" "topdir" "truncate" "unknown" "unpackdirs" "up" "uppercase" "urlbase" "urltofile" "var"
    "vareq" "varfn" "varfnarray" "varinlist" "varne" "varstrs" "varz" "whilestmt"
    "wnhypnymstoconcept" "wninit" "wnsensestoconcept" "wordindex" "wordpath" "writekb" "xaddlen"
    "xaddnvar" "xdump" "xinc" "xmlstr" "xrename"))

(pre_action
  (call_expression
    function: (identifier) @function.builtin))

; "$text", "$ostart" and the other special variable names
((string) @string.special
  (#match? @string.special "^\"\\$"))

((string) @string
  (#not-match? @string "^\"\\$"))

(escape_sequence) @constant.character.escape

(number) @constant.numeric.integer

(float) @constant.numeric.float

[
  "="
  "+"
  "-"
  "*"
  "/"
  "%"
  "%%"
  "<<"
  "&&"
  "||"
  "!"
  "=="
  "!="
  "<="
  ">="
  "++"
  "--"
] @operator

(binary_expression
  operator: [
    "<"
    ">"
  ] @operator)

(reference) @operator

"::" @punctuation.delimiter

[
  ","
  ";"
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

(pre_action
  [
    "<"
    ">"
  ] @punctuation.bracket)

(modifiers
  "*" @attribute)
