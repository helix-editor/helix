; Identifiers

[(NAME) (SYMBOLNAME)] @variable

(section
  .
  (NAME) @namespace)

; Operators

[
  "="
  "+="
  "-="
  "*="
  "/="
  "<<="
  ">>="
  "&="
  "|="
  "^="
  "*"
  "/"
  "%"
  "+"
  "-"
  "<<"
  ">>"
  "=="
  "!="
  "<="
  ">="
  "<"
  ">"
  "&"
  "^"
  "|"
  "&&"
  "||"
  "?"
] @operator

; Keywords

[
  "ABSOLUTE"
  "ADDR"
  "ALIGNOF"
  "ASSERT"
  "BYTE"
  "CONSTANT"
  "DATA_SEGMENT_ALIGN"
  "DATA_SEGMENT_END"
  "DATA_SEGMENT_RELRO_END"
  "DEFINED"
  "LOADADDR"
  "LOG2CEIL"
  "LONG"
  "MAX"
  "MIN"
  "NEXT"
  "QUAD"
  "SHORT"
  "SIZEOF"
  "SQUAD"
  "FILL"
  "SEGMENT_START"
] @function.builtin

[
  "CONSTRUCTORS"
  "CREATE_OBJECT_SYMBOLS"
  "LINKER_VERSION"
  "SIZEOF_HEADERS"
] @constant.builtin

[
  "AFTER"
  "ALIGN"
  "ALIGN_WITH_INPUT"
  "ASCIZ"
  "AS_NEEDED"
  "AT"
  "BEFORE"
  "BIND"
  "BLOCK"
  "COPY"
  "DSECT"
  "ENTRY"
  "EXCLUDE_FILE"
  "EXTERN"
  "extern"
  "FLOAT"
  "FORCE_COMMON_ALLOCATION"
  "FORCE_GROUP_ALLOCATION"
  "global"
  "GROUP"
  "HIDDEN"
  "HLL"
  "INCLUDE"
  "INFO"
  "INHIBIT_COMMON_ALLOCATION"
  "INPUT"
  "INPUT_SECTION_FLAGS"
  "KEEP"
  "l"
  "LD_FEATURE"
  "len"
  "LENGTH"
  "local"
  "MAP"
  "MEMORY"
  "NOCROSSREFS"
  "NOCROSSREFS_TO"
  "NOFLOAT"
  "NOLOAD"
  "o"
  "ONLY_IF_RO"
  "ONLY_IF_RW"
  "org"
  "ORIGIN"
  "OUTPUT"
  "OUTPUT_ARCH"
  "OUTPUT_FORMAT"
  "OVERLAY"
  "PHDRS"
  "PROVIDE"
  "PROVIDE_HIDDEN"
  "READONLY"
  "REGION_ALIAS"
  "REVERSE"
  "SEARCH_DIR"
  "SECTIONS"
  "SORT"
  "SORT_BY_ALIGNMENT"
  "SORT_BY_INIT_PRIORITY"
  "SORT_BY_NAME"
  "SORT_NONE"
  "SPECIAL"
  "STARTUP"
  "SUBALIGN"
  "SYSLIB"
  "TARGET"
  "TYPE"
  "VERSION"
] @keyword

; Delimiters

[
  ","
  ";"
  "&"
  ":"
  ">"
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

; Literals

(INT) @constant.numeric.integer

; Comment

(comment) @comment
