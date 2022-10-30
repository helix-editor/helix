(keyword_gist) @function.builtin
(keyword_btree) @function.builtin
(keyword_btree) @function.builtin
(keyword_hash) @function.builtin
(keyword_spgist) @function.builtin
(keyword_gin) @function.builtin
(keyword_brin) @function.builtin
(keyword_float) @function.builtin

(invocation
  name: (identifier) @function.builtin
  parameter: [(field)]? @variable.other.member)
  
(count
  name: (identifier) @function.builtin
  parameter: [(field)]? @variable.other.member)
  
(table_reference
  name: (identifier) @namespace)

(relation
  table_alias: (identifier) @variable.parameter)
  
(field
  name: (identifier) @variable.other.member)
  
(field
  table_alias: (identifier) @variable.parameter
  name: (identifier) @variable.other.member)


(comment) @comment

[
  "("
  ")"
] @punctuation.bracket

[
  ";"
  ","
  "."
] @punctuation.delimiter

[
  "*"
  "+"
  "-"
  "/"
  "%"
  "^"
  "||"  
  "="
  "<"
  "<="
  "!="
  ">="
  ">"
] @operator

[
  (keyword_null)
  (keyword_true)
  (keyword_false)
] @constant.builtin

(literal) @string

((literal) @constant.numeric
  (#match? @constant.numeric "^(-?\d*\.?\d*)$"))

[
  (keyword_select)
  (keyword_delete)
  (keyword_insert)
  (keyword_replace)
  (keyword_update)
  (keyword_into)
  (keyword_values)
  (keyword_set)
  (keyword_from)
  (keyword_left)
  (keyword_right)
  (keyword_inner)
  (keyword_outer)
  (keyword_cross)
  (keyword_join)
  (keyword_lateral)
  (keyword_on)
  (keyword_not)
  (keyword_order)
  (keyword_group)
  (keyword_partition)
  (keyword_by)
  (keyword_having)
  (keyword_desc)
  (keyword_asc)
  (keyword_limit)
  (keyword_offset)
  (keyword_primary)
  (keyword_create)
  (keyword_alter)
  (keyword_drop)
  (keyword_add)
  (keyword_table)
  (keyword_view)
  (keyword_materialized)
  (keyword_column)
  (keyword_key)
  (keyword_as)
  (keyword_distinct)
  (keyword_constraint)
  ; (keyword_count)
  (keyword_max)
  (keyword_min)
  (keyword_avg)
  (keyword_end)
  (keyword_force)
  (keyword_using)
  (keyword_use)
  (keyword_index)
  (keyword_for)
  (keyword_if)
  (keyword_exists)
  (keyword_auto_increment)
  (keyword_default)
  (keyword_cascade)
  (keyword_between)
  (keyword_window)
  (keyword_with)
  (keyword_no)
  (keyword_data)
  (keyword_type)
  (keyword_rename)
  (keyword_to)
  (keyword_schema)
  (keyword_owner)
  (keyword_temp)
  (keyword_temporary)
  (keyword_union)
  (keyword_all)
  (keyword_except)
  (keyword_intersect)
  (keyword_returning)
  (keyword_begin)
  (keyword_commit)
  (keyword_rollback)
  (keyword_transaction)
] @keyword

[
  (keyword_case)
  (keyword_when)
  (keyword_then)
  (keyword_else)
  (keyword_where)
] @keyword.control.conditional

[
  (keyword_in)
  (keyword_and)
  (keyword_or)
  (keyword_is)
] @keyword.operator

[
  (keyword_boolean)
  (keyword_smallserial)
  (keyword_serial)
  (keyword_bigserial)
  (keyword_smallint)
  (keyword_int)

  (bigint)
  (decimal)
  (numeric)
  (keyword_real)
  (double)

  (keyword_money)

  (char)
  (varchar)
  (keyword_text)

  (keyword_uuid)

  (keyword_json)
  (keyword_jsonb)
  (keyword_xml)

  (keyword_bytea)

  (keyword_date)
  (keyword_datetime)
  (keyword_timestamp)
  (keyword_timestamptz)

  (keyword_geometry)
  (keyword_geography)
  (keyword_box2d)
  (keyword_box3d)
] @type.builtin
