;; Math
[
 (displayed_equation)
 (inline_formula)
] @text.math

;; This highlights the whole environment like vimtex does
((environment
  (begin
   name: (word) @_env)) @text.math
   (#any-of? @_env
      "displaymath" "displaymath*"
      "equation" "equation*"
      "multline" "multline*"
      "eqnarray" "eqnarray*"
      "align" "align*"
      "array" "array*"
      "split" "split*"
      "alignat" "alignat*"
      "gather" "gather*"
      "flalign" "flalign*"))

[
  (generic_command_name)
  "\\newcommand"
  "\\renewcommand"
  "\\DeclareRobustCommand"
  "\\DeclareMathOperator"
  "\\newglossaryentry"
  "\\caption"
  "\\label"
  "\\newlabel"
  "\\color"
  "\\colorbox"
  "\\textcolor"
  "\\pagecolor"
  "\\definecolor"
  "\\definecolorset"
  "\\newtheorem"
  "\\declaretheorem"
  "\\newacronym"
] @function.macro

[
    "\\ref"
    "\\vref"
    "\\Vref"
    "\\autoref"
    "\\pageref"
    "\\cref"
    "\\Cref"
    "\\cref*"
    "\\Cref*"
    "\\namecref"
    "\\nameCref"
    "\\lcnamecref"
    "\\namecrefs"
    "\\nameCrefs"
    "\\lcnamecrefs"
    "\\labelcref"
    "\\labelcpageref"
    "\\crefrange"
    "\\crefrange"
    "\\Crefrange"
    "\\Crefrange"
    "\\crefrange*"
    "\\crefrange*"
    "\\Crefrange*"
    "\\Crefrange*"
] @function.macro

[
    "\\cite"
    "\\cite*"
    "\\Cite"
    "\\nocite"
    "\\citet"
    "\\citep"
    "\\citet*"
    "\\citep*"
    "\\citeauthor"
    "\\citeauthor*"
    "\\Citeauthor"
    "\\Citeauthor*"
    "\\citetitle"
    "\\citetitle*"
    "\\citeyear"
    "\\citeyear*"
    "\\citedate"
    "\\citedate*"
    "\\citeurl"
    "\\fullcite"
    "\\citeyearpar"
    "\\citealt"
    "\\citealp"
    "\\citetext"
    "\\parencite"
    "\\parencite*"
    "\\Parencite"
    "\\footcite"
    "\\footfullcite"
    "\\footcitetext"
    "\\textcite"
    "\\Textcite"
    "\\smartcite"
    "\\Smartcite"
    "\\supercite"
    "\\autocite"
    "\\Autocite"
    "\\autocite*"
    "\\Autocite*"
    "\\volcite"
    "\\Volcite"
    "\\pvolcite"
    "\\Pvolcite"
    "\\fvolcite"
    "\\ftvolcite"
    "\\svolcite"
    "\\Svolcite"
    "\\tvolcite"
    "\\Tvolcite"
    "\\avolcite"
    "\\Avolcite"
    "\\notecite"
    "\\notecite"
    "\\pnotecite"
    "\\Pnotecite"
    "\\fnotecite"
] @function.macro

[
    "\\ref"
    "\\vref"
    "\\Vref"
    "\\autoref"
    "\\pageref"
    "\\cref"
    "\\Cref"
    "\\cref*"
    "\\Cref*"
    "\\namecref"
    "\\nameCref"
    "\\lcnamecref"
    "\\namecrefs"
    "\\nameCrefs"
    "\\lcnamecrefs"
    "\\labelcref"
    "\\labelcpageref"
] @function.macro


[
    "\\crefrange"
    "\\crefrange"
    "\\Crefrange"
    "\\Crefrange"
    "\\crefrange*"
    "\\crefrange*"
    "\\Crefrange*"
    "\\Crefrange*"
] @function.macro


[
  "\\gls"
  "\\Gls"
  "\\GLS"
  "\\glspl"
  "\\Glspl"
  "\\GLSpl"
  "\\glsdisp"
  "\\glslink"
  "\\glstext"
  "\\Glstext"
  "\\GLStext"
  "\\glsfirst"
  "\\Glsfirst"
  "\\GLSfirst"
  "\\glsplural"
  "\\Glsplural"
  "\\GLSplural"
  "\\glsfirstplural"
  "\\Glsfirstplural"
  "\\GLSfirstplural"
  "\\glsname"
  "\\Glsname"
  "\\GLSname"
  "\\glssymbol"
  "\\Glssymbol"
  "\\glsdesc"
  "\\Glsdesc"
  "\\GLSdesc"
  "\\glsuseri"
  "\\Glsuseri"
  "\\GLSuseri"
  "\\glsuserii"
  "\\Glsuserii"
  "\\GLSuserii"
  "\\glsuseriii"
  "\\Glsuseriii"
  "\\GLSuseriii"
  "\\glsuseriv"
  "\\Glsuseriv"
  "\\GLSuseriv"
  "\\glsuserv"
  "\\Glsuserv"
  "\\GLSuserv"
  "\\glsuservi"
  "\\Glsuservi"
  "\\GLSuservi"
] @function.macro


[
  "\\acrshort"
  "\\Acrshort"
  "\\ACRshort"
  "\\acrshortpl"
  "\\Acrshortpl"
  "\\ACRshortpl"
  "\\acrlong"
  "\\Acrlong"
  "\\ACRlong"
  "\\acrlongpl"
  "\\Acrlongpl"
  "\\ACRlongpl"
  "\\acrfull"
  "\\Acrfull"
  "\\ACRfull"
  "\\acrfullpl"
  "\\Acrfullpl"
  "\\ACRfullpl"
  "\\acs"
  "\\Acs"
  "\\acsp"
  "\\Acsp"
  "\\acl"
  "\\Acl"
  "\\aclp"
  "\\Aclp"
  "\\acf"
  "\\Acf"
  "\\acfp"
  "\\Acfp"
  "\\ac"
  "\\Ac"
  "\\acp"
  "\\glsentrylong"
  "\\Glsentrylong"
  "\\glsentrylongpl"
  "\\Glsentrylongpl"
  "\\glsentryshort"
  "\\Glsentryshort"
  "\\glsentryshortpl"
  "\\Glsentryshortpl"
  "\\glsentryfullpl"
  "\\Glsentryfullpl"
] @function.macro

(comment) @comment

(bracket_group) @variable.parameter

[(math_operator) "="] @operator

[
  "\\usepackage"
  "\\documentclass"
  "\\input"
  "\\include"
  "\\subfile"
  "\\subfileinclude"
  "\\subfileinclude"
  "\\includegraphics"
  "\\addbibresource"
  "\\bibliography"
  "\\includesvg"
  "\\includeinkscape"
  "\\usepgflibrary"
  "\\usetikzlibrary"
] @keyword.control.import

[
  "\\part"
  "\\chapter"
  "\\section"
  "\\subsection"
  "\\subsubsection"
  "\\paragraph"
  "\\subparagraph"
] @type

"\\item" @punctuation.special

((word) @punctuation.delimiter
(#eq? @punctuation.delimiter "&"))

["$" "\\[" "\\]" "\\(" "\\)"] @punctuation.delimiter

(label_definition
 name: (_) @text.reference)
(label_reference
 label: (_) @text.reference)
(equation_label_reference
 label: (_) @text.reference)
(label_reference
 label: (_) @text.reference)
(label_number
 label: (_) @text.reference)

(citation
 key: (word) @text.reference)

(key_val_pair
  key: (_) @variable.parameter
  value: (_))

["[" "]" "{" "}"] @punctuation.bracket ;"(" ")" is has no special meaning in LaTeX

(chapter
  text: (brace_group) @markup.heading)

(part
  text: (brace_group) @markup.heading)

(section
  text: (brace_group) @markup.heading)

(subsection
  text: (brace_group) @markup.heading)

(subsubsection
  text: (brace_group) @markup.heading)

(paragraph
  text: (brace_group) @markup.heading)

(subparagraph
  text: (brace_group) @markup.heading)

((environment
  (begin
   name: (word) @_frame)
   (brace_group
        child: (text) @markup.heading))
 (#eq? @_frame "frame"))

((generic_command
  name:(generic_command_name) @_name
  arg: (brace_group
          (text) @markup.heading))
 (#eq? @_name "\\frametitle"))

;; Formatting

((generic_command
  name:(generic_command_name) @_name
  arg: (_) @markup.italic)
 (#eq? @_name "\\emph"))

((generic_command
  name:(generic_command_name) @_name
  arg: (_) @markup.italic)
 (#match? @_name "^(\\\\textit|\\\\mathit)$"))

((generic_command
  name:(generic_command_name) @_name
  arg: (_) @markup.bold)
 (#match? @_name "^(\\\\textbf|\\\\mathbf)$"))

((generic_command
  name:(generic_command_name) @_name
  .
  arg: (_) @markup.link.url)
 (#match? @_name "^(\\\\url|\\\\href)$"))

(ERROR) @error

[
  "\\begin"
  "\\end"
] @text.environment

(begin
 name: (_) @text.environment.name
  (#not-any-of? @text.environment.name
      "displaymath" "displaymath*"
      "equation" "equation*"
      "multline" "multline*"
      "eqnarray" "eqnarray*"
      "align" "align*"
      "array" "array*"
      "split" "split*"
      "alignat" "alignat*"
      "gather" "gather*"
      "flalign" "flalign*"))

(end
 name: (_) @text.environment.name
  (#not-any-of? @text.environment.name
      "displaymath" "displaymath*"
      "equation" "equation*"
      "multline" "multline*"
      "eqnarray" "eqnarray*"
      "align" "align*"
      "array" "array*"
      "split" "split*"
      "alignat" "alignat*"
      "gather" "gather*"
      "flalign" "flalign*"))
