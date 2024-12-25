;; General syntax
(ERROR) @error

(command_name) @function
(caption
  command: _ @function)

(key_value_pair
  key: (_) @variable.parameter
  value: (_))

[
  (comment)
  (line_comment)
  (block_comment)
  (comment_environment)
] @comment

[
  (brack_group)
  (brack_group_argc)
] @variable.parameter

[(operator) "="] @operator

"\\item" @punctuation.special

((word) @punctuation.delimiter
  (#eq? @punctuation.delimiter "&"))

["[" "]" "{" "}"] @punctuation.bracket ; "(" ")" has no syntactical meaning in LaTeX
(math_delimiter
  left_command: _ @punctuation.delimiter
  left_delimiter: _ @punctuation.delimiter
  right_command: _ @punctuation.delimiter
  right_delimiter: _ @punctuation.delimiter
)

;; General environments
(begin
  command: _ @function.builtin
  name: (curly_group_text (text) @function.macro))

(end
  command: _ @function.builtin
  name: (curly_group_text (text) @function.macro))

;; Definitions and references
(new_command_definition
  command: _ @function.macro
  declaration: (curly_group_command_name (_) @function))
(old_command_definition
  command: _ @function.macro
  declaration: (_) @function)
(let_command_definition
  command: _ @function.macro
  declaration: (_) @function)

(environment_definition
  command: _ @function.macro
  name: (curly_group_text (_) @constant))

(theorem_definition
  command: _ @function.macro
  name: (curly_group_text (_) @constant))

(paired_delimiter_definition
  command: _ @function.macro
  declaration: (curly_group_command_name (_) @function))

(label_definition
  command: _ @function.macro
  name: (curly_group_text (_) @label))
(label_reference_range
  command: _ @function.macro
  from: (curly_group_text (_) @label)
  to: (curly_group_text (_) @label))
(label_reference
  command: _ @function.macro
  names: (curly_group_text_list (_) @label))
(label_number
  command: _ @function.macro
  name: (curly_group_text (_) @label)
  number: (_) @markup.link.label)

(citation
  command: _ @function.macro
  keys: (curly_group_text_list) @string)

(glossary_entry_definition
  command: _ @function.macro
  name: (curly_group_text (_) @string))
(glossary_entry_reference
  command: _ @function.macro
  name: (curly_group_text (_) @string))

(acronym_definition
  command: _ @function.macro
  name: (curly_group_text (_) @string))
(acronym_reference
  command: _ @function.macro
  name: (curly_group_text (_) @string))

(color_definition
  command: _ @function.macro
  name: (curly_group_text (_) @string))
(color_reference
  command: _ @function.macro
  name: (curly_group_text (_) @string))

;; Math

(displayed_equation) @markup.raw.block
(inline_formula) @markup.raw.inline

(math_environment
  (begin
    command: _ @function.builtin
    name: (curly_group_text (text) @markup.raw)))

(math_environment
  (text) @markup.raw)

(math_environment
  (end
    command: _ @function.builtin
    name: (curly_group_text (text) @markup.raw)))

;; Sectioning
(title_declaration
  command: _ @namespace
  options: (brack_group (_) @markup.heading)?
  text: (curly_group (_) @markup.heading))

(author_declaration
  command: _ @namespace
  authors: (curly_group_author_list
             ((author)+ @markup.heading)))

(chapter
  command: _ @namespace
  toc: (brack_group (_) @markup.heading)?
  text: (curly_group (_) @markup.heading))

(part
  command: _ @namespace
  toc: (brack_group (_) @markup.heading)?
  text: (curly_group (_) @markup.heading))

(section
  command: _ @namespace
  toc: (brack_group (_) @markup.heading)?
  text: (curly_group (_) @markup.heading))

(subsection
  command: _ @namespace
  toc: (brack_group (_) @markup.heading)?
  text: (curly_group (_) @markup.heading))

(subsubsection
  command: _ @namespace
  toc: (brack_group (_) @markup.heading)?
  text: (curly_group (_) @markup.heading))

(paragraph
  command: _ @namespace
  toc: (brack_group (_) @markup.heading)?
  text: (curly_group (_) @markup.heading))

(subparagraph
  command: _ @namespace
  toc: (brack_group (_) @markup.heading)?
  text: (curly_group (_) @markup.heading))

;; Beamer frames
(generic_environment
  (begin
    name: (curly_group_text
            (text) @markup.heading)
    (#any-of? @markup.heading "frame"))
  .
  (curly_group (_) @markup.heading))

((generic_command
  command: (command_name) @_name
  arg: (curly_group
          (text) @markup.heading))
  (#eq? @_name "\\frametitle"))

;; Formatting
((generic_command
  command: (command_name) @_name
  arg: (curly_group (_) @markup.italic))
  (#eq? @_name "\\emph"))

((generic_command
  command: (command_name) @_name
  arg: (curly_group (_) @markup.italic))
  (#match? @_name "^(\\\\textit|\\\\mathit)$"))

((generic_command
  command: (command_name) @_name
  arg: (curly_group (_) @markup.bold))
  (#match? @_name "^(\\\\textbf|\\\\mathbf)$"))

((generic_command
  command: (command_name) @_name
  .
  arg: (curly_group (_) @markup.link.uri))
  (#match? @_name "^(\\\\url|\\\\href)$"))

;; File inclusion commands
(class_include
  command: _ @keyword.storage.type
  path: (curly_group_path) @string)

(package_include
  command: _ @keyword.storage.type
  paths: (curly_group_path_list) @string)

(latex_include
  command: _ @keyword.control.import
  path: (curly_group_path) @string)
(import_include
  command: _ @keyword.control.import
  directory: (curly_group_path) @string
  file: (curly_group_path) @string)

(bibtex_include
  command: _ @keyword.control.import
  path: (curly_group_path) @string)
(biblatex_include
  "\\addbibresource" @include
  glob: (curly_group_glob_pattern) @string.regex)

(graphics_include
  command: _ @keyword.control.import
  path: (curly_group_path) @string)
(tikz_library_import
  command: _ @keyword.control.import
  paths: (curly_group_path_list) @string)
