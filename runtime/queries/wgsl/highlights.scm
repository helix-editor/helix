(int_literal) @constant.numeric.integer
(float_literal) @constant.numeric.float
(bool_literal) @constant.builtin.boolean

[
  "bitcast"
  "discard"
  "enable"
  "fallthrough"
] @keyword

[
  "let"
  "override"
  "struct"
  "type"
  "var"
  (texel_format)
] @keyword.storage.type

[
  (access_mode)
  (address_space)
] @keyword.storage.modifier

"fn" @keyword.function

"return" @keyword.control.return

["," "." ":" ";"] @punctuation.delimiter

["(" ")" "[" "]" "{" "}"] @punctuation.bracket

[
  "break"
  "continue"
  "continuing"
] @keyword.control

[
  "loop"
  "for"
  "while"
] @keyword.control.repeat

[
  "if"
  "else"
  "switch"
  "case"
  "default"
] @keyword.control.conditional

[
  "!"
  "!="
  "%"
  "%="
  "&"
  "&&"
  "&="
  "*"
  "*="
  "+"
  "++"
  "+="
  "-"
  "--"
  "-="
  "->"
  "/"
  "/="
  "<"
  "<<"
  "<="
  "="
  "=="
  ">"
  ">="
  ">>"
  "@"
  "^"
  "^="
  "|"
  "|="
  "||"
  "~"
] @operator

(identifier) @variable

(function_declaration
  (identifier) @function)

(parameter
  (variable_identifier_declaration
    (identifier) @variable.parameter))

(struct_declaration
  (identifier) @type)

(struct_declaration
  (struct_member
    (variable_identifier_declaration
      (identifier) @variable.other.member)))

(type_constructor_or_function_call_expression
  (type_declaration (identifier) @function))

(type_declaration _ @type)

(attribute
  (identifier) @attribute)

(comment) @comment

; built-in wgsl functions: https://webgpufundamentals.org/webgpu/lessons/webgpu-wgsl-function-reference.html
(
  (identifier) @function.builtin
  (#any-of? @function.builtin 
    "abs"
    "abs"
    "acos"
    "acosh"
    "all"
    "any"
    "arrayLength"
    "asin"
    "asinh"
    "atan"
    "atan2"
    "atanh"
    "atomicAdd"
    "atomicLoad"
    "atomicStore"
    "bitcast"
    "ceil"
    "clamp"
    "cos"
    "cosh"
    "countLeadingZeros"
    "countOneBits"
    "countTrailingZeros"
    "cross"
    "degrees"
    "determinant"
    "distance"
    "dot"
    "dpdx"
    "dpdxCoarse"
    "dpdxFine"
    "dpdy"
    "dpdyCoarse"
    "dpdyFine"
    "exp"
    "exp2"
    "extractBits"
    "faceForward"
    "firstLeadingBit"
    "firstTrailingBit"
    "floor"
    "fma"
    "fract"
    "frexp"
    "fwidth"
    "fwidthCoarse"
    "fwidthFine"
    "gather_depth_compare"
    "gather_x_components"
    "insertBits"
    "inverseSqrt"
    "ldexp"
    "length"
    "log"
    "log2"
    "max"
    "min"
    "mix"
    "modf"
    "normalize"
    "pack2x16float"
    "pack2x16snorm"
    "pack2x16unorm"
    "pack4x8snorm"
    "pack4x8unorm"
    "pow"
    "quantizeToF16"
    "radians"
    "reflect"
    "refract"
    "reverseBits"
    "round"
    "saturate"
    "select"
    "sign"
    "sin"
    "sinh"
    "smoothstep"
    "sqrt"
    "step"
    "storageBarrier"
    "tan"
    "tanh"
    "textureDimensions"
    "textureGather"
    "textureGatherCompare"
    "textureLoad"
    "textureNumLayers"
    "textureNumLevels"
    "textureNumSamples"
    "textureSample"
    "textureSampleBaseClampToEdge"
    "textureSampleBias"
    "textureSampleCompare"
    "textureSampleCompareLevel"
    "textureSampleGrad"
    "textureSampleLevel"
    "textureStore"
    "transpose"
    "trunc"
    "unpack2x16float"
    "unpack2x16snorm"
    "unpack2x16unorm"
    "unpack4x8snorm"
    "unpack4x8unorm"
    "workgroupBarrier"
    "workgroupUniformLoad"
  )
)

(type_declaration ["<" ">"] @punctuation.bracket)
(variable_qualifier ["<" ">"] @punctuation.bracket)
