; Brace-delimited types and information-object syntax. Single-line nodes are a no-op.
[
  (SequenceType)
  (SetType)
  (ChoiceType)
  (EnumeratedType)
  (DefaultSyntax)
  (DefinedSyntax)
  (ExtensionAdditionGroup)
  (ObjectSet)
  (ValueSet)
  (SequenceValue)
  (SetValue)
  (SequenceOfValue)
  (SetOfValue)
  (ObjectIdentifierValue)
  (DefinitiveOID)
  (ParameterList)
  (ActualParameterList)
  (FullSpecification)
  (Constraint)
] @indent

; Re-align all assignments with the ModuleIdentifier
((Assignment
   (ModuleIdentifier) @anchor) @align)

; Only the named-list forms, not bare INTEGER / BIT STRING.
(IntegerType
  (NamedNumberList)) @indent

(BitStringType
  (NamedBitList)) @indent

; FROM-module lines hang one step under the imported symbols.
(SymbolList) @indent

; Branch/outdent only when the closer is first on the line.
[
  "}"
  "]"
  "]]"
  ";"
] @outdent
