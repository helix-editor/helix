[
  (label)
  (bb_ref)
] @label

[
  (comment)
  (multiline_comment)
] @comment

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  "<"
  ">"
] @punctuation.bracket

[
  ","
  ":"
  "|"
  "*"
] @punctuation.delimiter

[
  "="
  "x"
] @operator

[
  "true"
  "false"
] @constant.builtin.boolean

[
  "null"
  "_"
  "unknown-address"
] @constant.builtin

[
  (stack_object)
  (constant_pool_index)
  (jump_table_index)
  (var)
  (physical_register)
  (ir_block)
  (external_symbol)
  (global_var)
  (ir_local_var)
  (metadata_ref)
  (mnemonic)
] @variable

(low_level_type) @type

[
  (immediate_type)
  (primitive_type)
] @type.builtin

(number) @constant.numeric.integer
(float) @constant.numeric.float
(string) @string

(instruction name: _ @keyword.operator)

[
  "successors"
  "liveins"
  "pre-instr-symbol"
  "post-instr-symbol"
  "heap-alloc-marker"
  "pcsections"
  "mmra"
  "cfi-type"
  "debug-instr-number"
  "debug-location"
  "dbg-instr-ref"
  "mcsymbol"
  "tied-def"
  "target-flags"
  "vscale"
  "CustomRegMask"
  "same_value"
  "def_cfa_register"
  "restore"
  "undefined"
  "offset"
  "rel_offset"
  "def_cfa"
  "llvm_def_aspace_cfa"
  "register"
  "escape"
  "remember_state"
  "restore_state"
  "window_save"
  "negate_ra_sign_state"
  "intpred"
  "floatpred"
  "shufflemask"
  "liveout"
  "target-index"
  "blockaddress"
  "intrinsic"
  "load"
  "store"
  "unknown-size"
  "on"
  "from"
  "into"
  "align"
  "basealign"
  "addrspace"
  "call-entry"
  "custom"
  "constant-pool"
  "stack"
  "got"
  "jump-table"
  "syncscope"
  "machine-block-address-taken"
  "ir-block-address-taken"
  "landing-pad"
  "inlineasm-br-indirect-target"
  "ehfunclet-entry"
  "bb_id"
  "call-frame-size"
  "bbsections"
  "Exception"
  "Cold"

  (intpred)
  (floatpred)
  (memory_operand_flag)
  (atomic_ordering)
  (register_flag)
  (instruction_flag)
  (float_keyword)
] @keyword
