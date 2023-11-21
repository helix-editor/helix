; inherits: c

[
  "soa"
  "task"
  "launch"
  "unmasked"
  "template"
  "typename"
  (sync_expression)
] @keyword

[
  "in"
  "new"
  "delete"
] @keyword.operator

[
  "cdo"
  "cfor"
  "cwhile"
  "foreach"
  "foreach_tiled"
  "foreach_active"
  "foreach_unique"
] @repeat

[
  "cif"
] @conditional

[
  "varying"
  "uniform"
] @type.qualifier

"__regcall" @attribute

(overload_declarator name: _ @function)
(foreach_statement range_operator: _ @operator)

(short_vector ["<" ">"] @punctuation.bracket)
(soa_qualifier ["<" ">"] @punctuation.bracket)
(template_argument_list ["<" ">"] @punctuation.bracket)
(template_parameter_list ["<" ">"] @punctuation.bracket)

(llvm_identifier) @function.builtin

; built-in variables
((identifier) @variable.builtin
  (#any-of? @variable.builtin
   "programCount"
   "programIndex"
   "taskCount"
   "taskCount0"
   "taskCount1"
   "taskCount2"
   "taskIndex"
   "taskIndex0"
   "taskIndex1"
   "taskIndex2"
   "threadCount"
   "threadIndex"
   ))

; preprocessor constants
((identifier) @constant.builtin
  (#any-of? @constant.builtin
   "ISPC"
   "ISPC_FP16_SUPPORTED"
   "ISPC_FP64_SUPPORTED"
   "ISPC_LLVM_INTRINSICS_ENABLED"
   "ISPC_MAJOR_VERSION"
   "ISPC_MINOR_VERSION"
   "ISPC_POINTER_SIZE"
   "ISPC_TARGET_AVX"
   "ISPC_TARGET_AVX2"
   "ISPC_TARGET_AVX512KNL"
   "ISPC_TARGET_AVX512SKX"
   "ISPC_TARGET_AVX512SPR"
   "ISPC_TARGET_NEON"
   "ISPC_TARGET_SSE2"
   "ISPC_TARGET_SSE4"
   "ISPC_UINT_IS_DEFINED"
   "PI"
   "TARGET_ELEMENT_WIDTH"
   "TARGET_WIDTH"
   ))

; standard library built-in
((type_identifier) @type.builtin
  (#lua-match? @type.builtin "^RNGState"))

(call_expression
  function: (identifier) @function.builtin
  (#any-of? @function.builtin
   "abs"
   "acos"
   "all"
   "alloca"
   "and"
   "any"
   "aos_to_soa2"
   "aos_to_soa3"
   "aos_to_soa4"
   "asin"
   "assert"
   "assume"
   "atan"
   "atan2"
   "atomic_add_global"
   "atomic_add_local"
   "atomic_and_global"
   "atomic_and_local"
   "atomic_compare_exchange_global"
   "atomic_compare_exchange_local"
   "atomic_max_global"
   "atomic_max_local"
   "atomic_min_global"
   "atomic_min_local"
   "atomic_or_global"
   "atomic_or_local"
   "atomic_subtract_global"
   "atomic_subtract_local"
   "atomic_swap_global"
   "atomic_swap_local"
   "atomic_xor_global"
   "atomic_xor_local"
   "avg_down"
   "avg_up"
   "broadcast"
   "ceil"
   "clamp"
   "clock"
   "cos"
   "count_leading_zeros"
   "count_trailing_zeros"
   "doublebits"
   "exclusive_scan_add"
   "exclusive_scan_and"
   "exclusive_scan_or"
   "exp"
   "extract"
   "fastmath"
   "float16bits"
   "floatbits"
   "float_to_half"
   "float_to_half_fast"
   "float_to_srgb8"
   "floor"
   "frandom"
   "frexp"
   "half_to_float"
   "half_to_float_fast"
   "insert"
   "intbits"
   "invoke_sycl"
   "isnan"
   "ISPCAlloc"
   "ISPCLaunch"
   "ISPCSync"
   "lanemask"
   "ldexp"
   "log"
   "max"
   "memcpy"
   "memcpy64"
   "memmove"
   "memmove64"
   "memory_barrier"
   "memset"
   "memset64"
   "min"
   "none"
   "num_cores"
   "or"
   "packed_load_active"
   "packed_store_active"
   "packed_store_active2"
   "packmask"
   "popcnt"
   "pow"
   "prefetch_l1"
   "prefetch_l2"
   "prefetch_l3"
   "prefetch_nt"
   "prefetchw_l1"
   "prefetchw_l2"
   "prefetchw_l3"
   "print"
   "random"
   "rcp"
   "rcp_fast"
   "rdrand"
   "reduce_add"
   "reduce_equal"
   "reduce_max"
   "reduce_min"
   "rotate"
   "round"
   "rsqrt"
   "rsqrt_fast"
   "saturating_add"
   "saturating_div"
   "saturating_mul"
   "saturating_sub"
   "seed_rng"
   "select"
   "shift"
   "shuffle"
   "signbits"
   "sign_extend"
   "sin"
   "sincos"
   "soa_to_aos2"
   "soa_to_aos3"
   "soa_to_aos4"
   "sqrt"
   "streaming_load"
   "streaming_load_uniform"
   "streaming_store"
   "tan"
   "trunc"
   ))
