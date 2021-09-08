(function_item
  body: (_) @function.inside) @function.around

(struct_item
  body: (_) @class.inside) @class.around

(enum_item
  body: (_) @class.inside) @class.around

(union_item
  body: (_) @class.inside) @class.around

(trait_item
  body: (_) @class.inside) @class.around

(impl_item
  body: (_) @class.inside) @class.around

(mod_item
  body: (_) @class.inside) @class.around
