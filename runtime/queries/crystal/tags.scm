(class_def name: (constant) @name) @definition.class
(struct_def name: (constant) @name) @definition.struct
(c_struct_def name: (constant) @name) @definition.struct
(union_def name: (constant) @name) @definition.struct
(module_def name: (constant) @name) @definition.module
(lib_def name: (constant) @name) @definition.module
(enum_def name: (constant) @name) @definition.enum
(annotation_def name: (constant) @name) @definition.interface
(type_def name: (constant) @name) @definition.type
(method_def name: [(identifier) (operator)] @name) @definition.function
(abstract_method_def name: [(identifier) (operator)] @name) @definition.function
(fun_def name: [(constant) (identifier)] @name) @definition.function
(macro_def name: (identifier) @name) @definition.macro
(const_assign lhs: (constant) @name) @definition.constant
