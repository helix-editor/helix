; highlights.scm
;
; Highlighting queries for D code for use by Tree-Sitter.
;
; Copyright 2022 Garrett D'Amore
;
; Distributed under the MIT License.
; (See accompanying file LICENSE.txt or https://opensource.org/licenses/MIT)
; SPDX-License-Identifier: MIT

; these are listed first, because they override keyword queries
(identity_expression (in) @operator)
(identity_expression (is) @operator)

(storage_class) @keyword.storage

(function_declaration (identifier) @function)

(call_expression (identifier) @function)
(call_expression (type (identifier) @function))

(module_fqn) @namespace

[
    (abstract)
    (alias)
    (align)
    (asm)
    (assert)
    (auto)
    (cast)
    (const)
    (debug)
    (delete)
    (deprecated)
    (export)
    (extern)
    (final)
    (immutable)
    (in)
    (inout)
    (invariant)
    (is)
    (lazy)
    ; "macro" - obsolete
    (mixin)
    (module)
    (new)
    (nothrow)
    (out)
    (override)
    (package)
    (pragma)
    (private)
    (protected)
    (public)
    (pure)
    (ref)
    (scope)
    (shared)
    (static)
    (super)
    (synchronized)
    (template)
    (this)
    (throw)
    (typeid)
    (typeof)
    (unittest)
    (version)
    (with)
    (gshared)
    (traits)
    (vector)
    (parameters_)
] @keyword

[
    (class)
    (struct)
    (interface)
    (union)
    (enum)
    (function)
    (delegate)
] @keyword.storage.type

[
    (break)
    (case)
    (catch)
    (continue)
    (do)
    (default)
    (finally)
    (else)
    (goto)
    (if)
    (switch)
    (try)
] @keyword.control

(return) @keyword.control.return

(import) @keyword.control.import

[
    (for)
    (foreach)
    (foreach_reverse)
    (while)
] @keyword.control.repeat

[
    (not_in)
    (not_is)
    "/="
    "/"
    ".."
    "..."
    "&"
    "&="
    "&&"
    "|"
    "|="
    "||"
    "-"
    "-="
    "--"
    "+"
    "+="
    "++"
    "<"
    "<="
    "<<"
    "<<="
    ">"
    ">="
    ">>="
    ">>>="
    ">>"
    ">>>"
    "!"
    "!="
    "?"
    "$"
    "="
    "=="
    "*"
    "*="
    "%"
    "%="
    "^"
    "^="
    "^^"
    "^^="
    "~"
    "~="
    "@"
    "=>"
] @operator

[
    "("
    ")"
    "["
    "]"
] @punctuation.bracket

[
    ";"
    "."
    ":"
    ","
] @punctuation.delimiter

[
    (true)
    (false)
] @constant.builtin.boolean

(null) @constant.builtin

(special_keyword) @constant.builtin

(directive) @keyword.directive
(shebang) @keyword.directive

(comment) @comment

[
    (void)
    (bool)
    (byte)
    (ubyte)
    (char)
    (short)
    (ushort)
    (wchar)
    (dchar)
    (int)
    (uint)
    (long)
    (ulong)
    (real)
    (double)
] @type.builtin

[
    (cent)
    (ucent)
    (ireal)
    (idouble)
    (ifloat)
    (creal)
    (double)
    (cfloat)
] @warning ; these types are deprecated

(identifier) @variable

(label (identifier) @label)
(goto_statement (goto) @keyword (identifier) @label)

(string_literal) @string
(int_literal) @constant.numeric.integer
(float_literal) @constant.numeric.float
(char_literal) @constant.character
(at_attribute) @attribute

; everything after __EOF_ is plain text
(end_file) @ui.text
