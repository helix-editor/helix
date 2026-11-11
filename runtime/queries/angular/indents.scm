; inherits: html

; --- Control Flow Blocks ---

; All Angular control flow statement bodies: @if { } @else { } @for { }
; @empty { } @switch { } @case { } @default { } @defer { }
; @placeholder { } @loading { } @error { }
(statement_block) @indent
; Closing brace of any statement block — dedents back to block opener level
(statement_block "}" @outdent)

; @switch (expr) { ... } — indents case/default children one level
(switch_statement) @indent
; Closing brace of the switch body — dedents back to @switch level
(switch_body "}" @outdent)
