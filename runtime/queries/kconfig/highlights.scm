[
  "source"
  "osource"
  "rsource"
  "orsource"
] @keyword.control.import

[
  "mainmenu"
  "config"
  "configdefault"
  "menuconfig"
  "choice"
  "endchoice"
  "comment"
  "menu"
  "endmenu"
  "prompt"
  "default"
  "range"
  "help"
  (optional)
  (modules)
] @keyword

[
  "if"
  "endif"
  "depends on"
  "select"
  "imply"
  "visible if"
] @keyword.control.conditional

[
  "def_bool"
  "def_tristate"
] @keyword.function

[
  "||"
  "&&"
  "="
  "!="
  "<"
  ">"
  "<="
  ">="
  "!"
] @operator

[
  "bool"
  "tristate"
  "int"
  "hex"
  "string"
] @type.builtin

[ "(" ")" ] @punctuation.bracket

(macro_variable ["$(" ")"] @punctuation.special)

(symbol) @variable

[
  (string)
  (macro_content)
  (text)
] @string

(config name: (name (symbol) @constant))
(configdefault name: (name (symbol) @constant))
(menuconfig name: (name (symbol) @constant))
(choice name: (name (symbol) @constant))

((symbol) @constant
  (#match? @constant "[A-Z0-9]+"))

(mainmenu name: (string) @markup.heading)
(comment_entry name: (string) @markup.heading)
(menu name: (string) @markup.heading)

(source (string) @string.special.url @string.special)

(comment) @comment
