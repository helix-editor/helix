((call
   target: (identifier) @_keyword
   (arguments
     [
       (call target: (identifier) @definition.function)
       ; function has a guard
       (binary_operator
         left: (call target: (identifier) @definition.function))
     ]))
 (#any-of? @_keyword "def" "defdelegate" "defguard" "defguardp" "defmacro" "defmacrop" "defn" "defnp" "defp"))
