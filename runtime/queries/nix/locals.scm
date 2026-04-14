; When using @local.reference, tree-sitter seems to apply the scope from the
; identifier it has looked up, which makes sense for most languages.
;
; However, we want to highlight things as functions based on their call-site,
; not their definition. TS's support for tracking locals impedes our ability to
; get the highlighting we want.
;
; Also, TS doesn't seem to support scoping as implemented in languages with
; lazy let bindings, which results in syntax highlighting / goto-reference
; results that depend on the order of definitions. That is counter to the
; semantics of Nix.
;
; So for now we'll opt for not having any locals queries.
;
; See: https://github.com/tree-sitter/tree-sitter/issues/918
