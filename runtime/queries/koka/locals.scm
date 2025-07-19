(modulebody) @local.scope

(block) @local.scope

(pattern
  (identifier
    (varid) @local.definition))

(decl
  (apattern
    (pattern
      (identifier
        (varid) @local.definition))))

(puredecl
  (funid
    (identifier
      (varid) @local.definition)))

(puredecl
  (binder
    (identifier
      (varid) @local.definition)))

(decl
  (binder
    (identifier
      (varid) @local.definition)))

(identifier (varid) @local.reference)
