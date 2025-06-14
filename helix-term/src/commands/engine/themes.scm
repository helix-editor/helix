(require-builtin helix/core/themes)
(require-builtin helix/components)

(provide attribute
         type
         type.builtin
         type.parameter
         type.enum
         type.enum.variant
         constructor
         constant
         constant.builtin
         constant.builtin.boolean
         constant.character
         constant.character.escape
         constant.numeric
         constant.numeric.integer
         constant.numeric.float
         string
         string.regexp
         string.special
         string.special.path
         string.special.url
         string.special.symbol
         comment
         comment.line
         comment.block
         comment.block.documentation
         variable
         variable.builtin
         variable.parameter
         variable.other
         variable.other.member
         variable.other.member.private
         label
         punctuation
         punctuation.delimiter
         punctuation.bracket
         punctuation.special
         keyword
         keyword.control
         keyword.control.conditional
         keyword.control.repeat
         keyword.control.import
         keyword.control.return
         keyword.control.exception
         keyword.operator
         keyword.directive
         keyword.function
         keyword.storage
         keyword.storage.type
         keyword.storage.modifier
         operator
         function
         function.builtin
         function.method
         function.method.private
         function.macro
         function.special
         tag
         tag.builtin
         namespace
         special
         markup
         markup.heading
         markup.heading.marker
         markup.heading.marker.1
         markup.heading.marker.2
         markup.heading.marker.3
         markup.heading.marker.4
         markup.heading.marker.5
         markup.heading.marker.6
         markup.list
         markup.list.unnumbered
         markup.list.numbered
         markup.list.checked
         markup.list.unchecked
         markup.bold
         markup.italic
         markup.strikethrough
         markup.link
         markup.link.url
         markup.link.label
         markup.link.text
         markup.quote
         markup.raw
         markup.raw.inline
         markup.raw.block
         diff
         diff.plus
         diff.plus.gutter
         diff.minus
         diff.minus.gutter
         diff.delta
         diff.delta.moved
         diff.delta.conflict
         diff.delta.gutter
         markup.normal.completion
         markup.normal.hover
         markup.heading.completion
         markup.heading.hover
         markup.raw.inline.completion
         markup.raw.inline.hover
         ui.background
         ui.background.separator
         ui.cursor
         ui.cursor.insert
         ui.cursor.select
         ui.cursor.match
         ui.cursor.primary
         ui.cursor.primary.normal
         ui.cursor.primary.insert
         ui.cursor.primary.select
         ui.debug.breakpoint
         ui.debug.active
         ui.gutter
         ui.gutter.selected
         ui.highlight.frameline
         ui.linenr
         ui.linenr.selected
         ui.statusline
         ui.statusline.inactive
         ui.statusline.normal
         ui.statusline.insert
         ui.statusline.select
         ui.statusline.separator
         ui.bufferline
         ui.bufferline.active
         ui.bufferline.background
         ui.popup
         ui.popup.info
         ui.window
         ui.help
         ui.text
         ui.text.focus
         ui.text.inactive
         ui.text.info
         ui.virtual.ruler
         ui.virtual.whitespace
         ui.virtual.indent-guide
         ui.virtual.inlay-hint
         ui.virtual.inlay-hint.parameter
         ui.virtual.inlay-hint.type
         ui.virtual.wrap
         ui.virtual.jump-label
         ui.menu
         ui.menu.selected
         ui.menu.scroll
         ui.selection
         ui.selection.primary
         ui.highlight
         ui.cursorline
         ui.cursorline.primary
         ui.cursorline.secondary
         ui.cursorcolumn.primary
         ui.cursorcolumn.secondary
         warning
         error
         info
         hint
         diagnostic
         diagnostic.hint
         diagnostic.info
         diagnostic.warning
         diagnostic.error
         diagnostic.unnecessary
         diagnostic.deprecated)

(provide hashmap->theme
         register-theme
         theme-style
         theme-set-style!
         string->color)

;;@doc
;; Register this theme with helix for use
(define (register-theme theme)
  (add-theme! *helix.cx* theme))

(define-syntax theme-func
  (syntax-rules ()
    [(_ scope doc-string)
     (@doc doc-string
           (define (scope theme style)
             (theme-set-style! theme (quote scope) style)
             theme))]

    [(_ scope)
     (define (scope theme style)
       (theme-set-style! theme (quote scope) style)
       theme)]))

(theme-func attribute "Class attributes, HTML tag attributes")
(theme-func type "Types")
(theme-func type.builtin "Primitive types provided by the language (`int`, `usize`)")
(theme-func type.parameter "Generic type parameters (`T`)")
(theme-func type.enum "Enum usage")
(theme-func type.enum.variant "Enum variant")
(theme-func constructor "Constructor usage")
(theme-func constant "Constants usage")
(theme-func constant.builtin
            "Special constants provided by the language (`true`, `false`, `nil`, etc)")
(theme-func constant.builtin.boolean "A special case for highlighting individual booleans")
(theme-func constant.character "Character usage")
(theme-func constant.character.escape "Highlighting individual escape characters")
(theme-func constant.numeric "Numbers")
(theme-func constant.numeric.integer "Integers")
(theme-func constant.numeric.float "Floats")
(theme-func string "Highlighting strings")
(theme-func string.regexp "Highlighting regular expressions")
(theme-func string.special "Special strings")
(theme-func string.special.path "Highlighting paths")
(theme-func string.special.url "Highlighting URLs")
(theme-func string.special.symbol "Erlang/Elixir atoms, Ruby symbols, Clojure keywords")
(theme-func comment "Highlighting comments")
(theme-func comment.line "Single line comments (`//`)")
(theme-func comment.block "Block comments (`/* */`)")
(theme-func comment.block.documentation "Documentation comments (e.g. `///` in Rust)")
(theme-func variable "Variables")
(theme-func variable.builtin "Reserved language variables (`self`, `this`, `super`, etc.)")
(theme-func variable.parameter "Function parameters")
(theme-func variable.other "Other variables")
(theme-func variable.other.member "Fields of composite data types (e.g. structs, unions)")
(theme-func variable.other.member.private
            "Private fields that use a unique syntax (currently just EMCAScript-based languages)")

(theme-func label "Highlighting labels")
(theme-func punctuation "Highlighting punctuation")
(theme-func punctuation.delimiter "Commas, colon")
(theme-func punctuation.bracket "Parentheses, angle brackets, etc.")
(theme-func punctuation.special "String interpolation brackets")

(theme-func keyword "Highlighting keywords")
(theme-func keyword.control "Control keywords")
(theme-func keyword.control.conditional "if, else")
(theme-func keyword.control.repeat "for, while, loop")
(theme-func keyword.control.import "import, export")
(theme-func keyword.control.return "return keyword")
(theme-func keyword.control.exception "exception keyword")

(theme-func keyword.operator "or, in")
(theme-func keyword.directive "Preprocessor directives (`#if` in C)")
(theme-func keyword.function "fn, func")
(theme-func keyword.storage "Keywords describing how things are stored")
(theme-func keyword.storage.type "The type of something, `class`, `function`, `var`, `let`, etc")
(theme-func keyword.storage.modifier "Storage modifiers like `static`, `mut`, `const`, `ref`, etc")

(theme-func operator "Operators such as `||`, `+=`, `>`, etc")
(theme-func function "Highlighting function calls")
(theme-func function.builtin "Builtin functions")
(theme-func function.method "Calling methods")
(theme-func function.method.private
            "Private methods that use a unique syntax (currently just ECMAScript-based languages)")
(theme-func function.macro "Highlighting macros")
(theme-func function.special "Preprocessor in C")

(theme-func tag "Tags (e.g. <body> in HTML)")
(theme-func tag.builtin "Builtin tags")

(theme-func namespace)
(theme-func special)
(theme-func markup "Highlighting markdown")
(theme-func markup.heading "Markdown heading")
(theme-func markup.heading.marker "Markdown heading marker")
(theme-func markup.heading.marker.1 "Markdown heading text h1")
(theme-func markup.heading.marker.2 "Markdown heading text h2")
(theme-func markup.heading.marker.3 "Markdown heading text h3")
(theme-func markup.heading.marker.4 "Markdown heading text h4")
(theme-func markup.heading.marker.5 "Markdown heading text h5")
(theme-func markup.heading.marker.6 "Markdown heading text h6")

(theme-func markup.list "Markdown lists")
(theme-func markup.list.unnumbered "Unnumbered markdown lists")
(theme-func markup.list.numbered "Numbered markdown lists")
(theme-func markup.list.checked "Checked markdown lists")
(theme-func markup.list.unchecked "Unchecked markdown lists")

(theme-func markup.bold "Markdown bold")
(theme-func markup.italic "Markdown italics")
(theme-func markup.strikethrough "Markdown strikethrough")
(theme-func markup.link "Markdown links")
(theme-func markup.link.url "URLs pointed to by links")
(theme-func markup.link.label "non-URL link references")
(theme-func markup.link.text "URL and image descriptions in links")
(theme-func markup.quote "Markdown quotes")
(theme-func markup.raw "Markdown raw")
(theme-func markup.raw.inline "Markdown inline raw")
(theme-func markup.raw.block "Markdown raw block")

(theme-func diff "Version control changes")
(theme-func diff.plus "Version control additions")
(theme-func diff.plus.gutter "Version control addition gutter indicator")
(theme-func diff.minus "Version control deletions")
(theme-func diff.minus.gutter "Version control deletion gutter indicator")
(theme-func diff.delta "Version control modifications")
(theme-func diff.delta.moved "Renamed or moved files/changes")
(theme-func diff.delta.conflict "Merge conflicts")
(theme-func diff.delta.gutter "Gutter indicator")

(theme-func markup.normal.completion "For completion doc popup UI")
(theme-func markup.normal.hover "For hover popup UI")
(theme-func markup.heading.completion "For completion doc popup UI")
(theme-func markup.heading.hover "For hover popup UI")
(theme-func markup.raw.inline.completion "For completion doc popup UI")
(theme-func markup.raw.inline.hover "For hover popup UI")

(theme-func ui.background)
(theme-func ui.background.separator "Picker separator below input line")
(theme-func ui.cursor)
(theme-func ui.cursor.normal)
(theme-func ui.cursor.insert)
(theme-func ui.cursor.select)
(theme-func ui.cursor.match "Matching bracket etc.")
(theme-func ui.cursor.primary "Cursor with primary selection")
(theme-func ui.cursor.primary.normal)
(theme-func ui.cursor.primary.insert)
(theme-func ui.cursor.primary.select)

(theme-func ui.debug.breakpoint "Breakpoint indicator, found in the gutter")
(theme-func ui.debug.active
            "Indicator for the line at which debugging execution is paused at, found in the gutter")
(theme-func ui.gutter "Gutter")
(theme-func ui.gutter.selected "Gutter for the line the cursor is on")
(theme-func ui.highlight.frameline "Line at which debugging execution is paused at")
(theme-func ui.linenr "Line numbers")
(theme-func ui.linenr.selected "Line number for the line the cursor is on")
(theme-func ui.statusline "Statusline")
(theme-func ui.statusline.inactive "Statusline (unfocused document)")
(theme-func ui.statusline.normal
            "Statusline mode during normal mode (only if editor.color-modes is enabled)")
(theme-func ui.statusline.insert
            "Statusline mode during insert mode (only if editor.color-modes is enabled)")
(theme-func ui.statusline.select
            "Statusline mode during select mode (only if editor.color-modes is enabled)")

(theme-func ui.statusline.separator "Separator character in statusline")
(theme-func ui.bufferline "Style for the buffer line")
(theme-func ui.bufferline.active "Style for the active buffer in buffer line")
(theme-func ui.bufferline.background "Style for the bufferline background")
(theme-func ui.popup "Documentation popups (e.g. Space + k)")
(theme-func ui.popup.info "Prompt for multiple key options")
(theme-func ui.window "Borderline separating splits")
(theme-func ui.help "Description box for commands")
(theme-func ui.text "Default text style, command prompts, popup text, etc.")
(theme-func ui.text.focus "The currently selected line in the picker")
(theme-func ui.text.inactive "Same as ui.text but when the text is inactive (e.g. suggestions)")
(theme-func ui.text.info "The key: command text in ui.popup.info boxes")
(theme-func ui.virtual.ruler "Ruler columns (see the editor.rules config)")
(theme-func ui.virtual.whitespace "Visible whitespace characters")
(theme-func ui.virtual.indent-guide "Vertical indent width guides")
(theme-func ui.virtual.inlay-hint "Default style for inlay hints of all kinds")
(theme-func ui.virtual.inlay-hint.parameter
            "Style for inlay hints of kind `parameter` (LSPs are not rquired to set a kind)")
(theme-func ui.virtual.inlay-hint.type
            "Style for inlay hints of kind `type` (LSPs are not required to set a kind)")
(theme-func ui.virtual.wrap "Soft-wrap indicator (see the editor.soft-wrap config)")
(theme-func ui.virtual.jump-label "Style for virtual jump labels")
(theme-func ui.menu "Code and command completion menus")
(theme-func ui.menu.selected "Selected autocomplete item")
(theme-func ui.menu.scroll "fg sets thumb color, bg sets track color of scrollbar")
(theme-func ui.selection "For selections in the editing area")
(theme-func ui.selection.primary)
(theme-func ui.highlight "Highlighted lines in the picker preview")
(theme-func ui.cursorline "The line of the cursor (if cursorline is enabled)")
(theme-func ui.cursorline.primary "The line of the primary cursor (if cursorline is enabled)")
(theme-func ui.cursorline.secondary "The line of the secondary cursor (if cursorline is enabled)")
(theme-func ui.cursorcolumn.primary "The column of the primary cursor (if cursorcolumn is enabled)")
(theme-func ui.cursorcolumn.secondary
            "The column of the secondary cursor (if cursorcolumn is enabled)")

(theme-func warning "Diagnostics warning (gutter)")
(theme-func error "Diagnostics error (gutter)")
(theme-func info "Diagnostics info (gutter)")
(theme-func hint "Diagnostics hint (gutter)")

(theme-func diagnostic "Diagnostics fallback style (editing area)")
(theme-func diagnostic.hint "Diagnostics hint (editing area)")
(theme-func diagnostic.info "Diagnostics info (editing area)")
(theme-func diagnostic.warning "Diagnostics warning (editing area)")
(theme-func diagnostic.error "Diagnostics error (editing area)")
(theme-func diagnostic.unnecessary "Diagnostics with unnecessary tag (editing area)")
(theme-func diagnostic.deprecated "Diagnostics with deprecated tag (editing area)")
