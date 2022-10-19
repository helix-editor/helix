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

[
    (abstract)
    (alias)
    (align)
    (asm)
    (assert)
    (auto)
    (cast)
    (class)
    (const)
    (debug)
    (delegate)
    (delete)
    (deprecated)
    (enum)
    (export)
    (extern)
    (final)
    (function)
    (immutable)
    (import)
    (in)
    (inout)
    (interface)
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
    (struct)
    (super)
    (synchronized)
    (template)
    (this)
    (throw)
    (typeid)
    (typeof)
    (union)
    (unittest)
    (version)
    (with)
    (gshared)
    (traits)
    (vector)
    (parameters_)
] @keyword

[
    (break)
    (case)
    (catch)
    (continue)
    (do)
    (default)
    (finally)
    (else)
    (for)
    (foreach)
    (foreach_reverse)
    (goto)
    (if)
    (switch)
    (try)
    (return)
    (while)
] @keyword.control

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
    "("
    ")"
    "["
    "]"
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
    ";"
    "."
    ":"
    ","
] @punctuation.delimiter

[
    (null)
    (true)
    (false)
] @constant.language

(special_keyword) @constant.language

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
] @type

[
    (cent)
    (ucent)
    (ireal)
    (idouble)
    (ifloat)
    (creal)
    (double)
    (cfloat)
] @type.deprecated

(label (identifier) @label)
(goto_statement (goto) @keyword (identifier) @label)

(string_literal) @string
(int_literal) @number
(float_literal) @number
(char_literal) @number
(identifier) @variable
(at_attribute) @property

; everything after __EOF_ is plain text
(end_file) @text
