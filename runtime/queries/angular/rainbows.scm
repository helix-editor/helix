; inherits: html

; All bracket-like delimiters get rainbow depth coloring:
; ( )       — expression grouping, function call args, @if/@for/@switch conditions
; [ ]       — property bindings [prop]="val", array literals, bracket access
; [( )]     — two-way bindings [(ngModel)]="val"
; [@        — animation bindings [@fadeIn]="trigger" (closes with ])
; {{ }}     — interpolation delimiters {{ expr }}
; { }       — control flow statement blocks @if (c) { ... }
["(" ")" "[" "]" "[(" ")]" "[@" "{{" "}}" "{" "}"] @rainbow.bracket
