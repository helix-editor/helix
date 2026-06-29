; See runtime/queries/ecma/README.md for more info.

; inherits: ecma,_javascript

; `using` / `await using` resource-management declarations (TC39). Kept here
; rather than in shared ecma/ because the pinned typescript grammar has no
; `using` token yet (move to ecma when typescript gains it).
"using" @keyword.storage.modifier
