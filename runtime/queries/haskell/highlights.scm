;; ----------------------------------------------------------------------------
;; Literals and comments

(integer) @constant.numeric.integer
(exp_negation) @constant.numeric.integer
(exp_literal (float)) @constant.numeric.float
(char) @constant.character
(string) @string

(exp_apply
 (exp_name
  (variable) @function.builtin
   (#any-of? @function.builtin
    ; built in functions from the Haskell prelude (https://hackage.haskell.org/package/base-4.21.0.0/docs/Prelude.html)
    ; basic data types
    "not"
    "maybe"
    "either"

    ; tuples
    "fst"
    "snd"
    "curry"
    "uncurry"

    ; Ord
    "compare"
    "min"
    "max"

    ; Enum
    "succ"
    "pred"
    "toEnum"
    "fromEnum"
    "enumFrom"
    "enumFromThen"
    "enumFromThenTo"

    ; Num
    "negate"
    "abs"
    "signum"
    "fromInteger"

    ; Real
    "toRational"

    ; Integral
    "quot"
    "rem"
    "div"
    "mod"
    "quotRem"
    "divMod"
    "toInteger"

    ; Fractional
    "recip"
    "fromRational"

    ; Floating
    "exp"
    "log"
    "sqrt"
    "logBase"
    "sin"
    "cos"
    "tan"
    "asin"
    "acos"
    "atan"
    "sinh"
    "cosh"
    "tanh"
    "asinh"
    "acosh"
    "atanh"

    ; RealFrac
    "properFraction"
    "truncate"
    "round"
    "ceiling"
    "floor"

    ; RealFloat
    "floatRadix"
    "floatDigits"
    "floatRange"
    "decodeFloat"
    "encodeFloat"
    "exponent"
    "significand"
    "scaleFloat"
    "isNaN"
    "isInfinite"
    "isDenormalized"
    "isNegativeZero"
    "isIEEE"
    "atan2"

    ; Numeric functions
    "subtract"
    "even"
    "odd"
    "gcd"
    "lcm"
    "fromIntegral"
    "realToFrac"

    ; Monoid
    "mempty"
    "mconcat"
    "mappend"

    ; Functor
    "fmap"

    ; Applicative
    "liftA2"
    "pure"
    
    ; Monad
    "return"

    ; MonadFail
    "fail"
    "mapM_"
    "sequence_"

    ; Foldable
    "foldMap"
    "foldr"
    "foldl"
    "foldl'"
    "foldr1"
    "foldl1"
    "elem"
    "maximum"
    "minimum"
    "sum"
    "product"

    ; Traversable
    "traverse"
    "sequenceA"
    "mapM"
    "sequence"

    ; miscellaneous
    "id"
    "const"
    "flip"
    "until"
    "asTypeOf"
    "error"
    "errorWithoutStackTrace"
    "undefined"

    ; List
    "map"
    "filter"
    "head"
    "last"
    "tail"
    "init"
    "null"
    "length"
    "reverse"

    ; Foldable
    "and"
    "or"
    "any"
    "all"
    "concat"
    "concatMap"

    ; Building lists
    "scanl"
    "scanl1"
    "scanr"
    "scanr1"

    ; Infinite lists
    "iterate"
    "repeat"
    "replicate"
    "cycle"

    ; Sublists
    "take"
    "drop"
    "takeWhile"
    "dropWhile"
    "span"
    "break"
    "splitAt"

    ; Searching lists
    "notElem"
    "lookup"

    ; zipping and unzipping
    "zip"
    "zip3"
    "zipWith"
    "zipWith3"
    "unzip"
    "unzip3"

    ; String
    "lines"
    "words"
    "unlines"
    "unwords"

    ; Converting to String
    "show"
    "showList"
    "shows"
    "showChar"
    "showString"
    "showParen"

    ; Converting from String
    "readsPrec"
    "readList"
    "reads"
    "readParen"
    "read"
    "lex"

    ; Input and output
    "putChar"
    "putStr"
    "putStrLn"
    "print"
    "getChar"
    "getLine"
    "getContents"
    "interact"

    ; Files 
    "readFile"
    "writeFile"
    "appendFile"
    "readIO"
    "readLn"

    ; Exception handling
    "ioError"
    "userError")
  )
)


(con_unit) @constant.builtin ; unit, as in ()

(comment) @comment


;; ----------------------------------------------------------------------------
;; Punctuation

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
] @punctuation.bracket

[
  (comma)
  ";"
] @punctuation.delimiter


;; ----------------------------------------------------------------------------
;; Keywords, operators, includes

[
  "forall"
  "∀"
] @keyword.control.repeat

(pragma) @constant.macro

[
  "if"
  "then"
  "else"
  "case"
  "of"
] @keyword.control.conditional

[
  "import"
  "qualified"
  "module"
] @keyword.control.import

[
  (operator)
  (constructor_operator)
  (type_operator)
  (tycon_arrow)
  (qualified_module)  ; grabs the `.` (dot), ex: import System.IO
  (all_names)
  (wildcard)
  "="
  "|"
  "::"
  "=>"
  "->"
  "<-"
  "\\"
  "`"
  "@"
] @operator

(module) @namespace

[
  (where)
  "let"
  "in"
  "class"
  "instance"
  "data"
  "newtype"
  "family"
  "type"
  "as"
  "hiding"
  "deriving"
  "via"
  "stock"
  "anyclass"
  "do"
  "mdo"
  "rec"
  "infix"
  "infixl"
  "infixr"
] @keyword


;; ----------------------------------------------------------------------------
;; Functions and variables

(signature name: (variable) @type)
(function
  name: (variable) @function
  patterns: (patterns))
((signature (fun)) . (function (variable) @function))
((signature (context (fun))) . (function (variable) @function))
((signature (forall (context (fun)))) . (function (variable) @function))

(exp_infix (variable) @operator)  ; consider infix functions as operators

(exp_infix (exp_name) @function)
(exp_apply . (exp_name (variable) @function))
(exp_apply . (exp_name (qualified_variable (variable) @function)))

(variable) @variable
(pat_wildcard) @variable

;; ----------------------------------------------------------------------------
;; Types

(type) @type
(type_variable) @type.parameter

(constructor) @constructor

; True or False
((constructor) @_bool (#match? @_bool "(True|False)")) @constant.builtin.boolean

;; ----------------------------------------------------------------------------
;; Quasi-quotes

(quoter) @function
; Highlighting of quasiquote_body is handled by injections.scm
