; inherits: json

; Inject bash in package.scripts
; e.g.
; ```
; {
;   "scripts": {
;     "build": "vite build && tsc --project tsconfig.server.json",
;               ^                                              ^
;               └──────────────────────────────────────────────┘
;   }
; }
; ```
((pair
  key: (string (string_content) @_scripts_key (#eq? @_scripts_key "scripts"))
  value: (object
    (pair
      value:
        (string (string_content) @injection.content))))
  (#set! injection.language "bash"))
