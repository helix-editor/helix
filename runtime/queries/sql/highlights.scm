(comment) @comment

[
  "("
  ")"
] @punctuation.bracket

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

(set_schema schema: (identifier) @namespace)
(table_reference schema: (identifier) @namespace)
(table_expression schema: (identifier) @namespace)
(all_fields schema: (identifier) @namespace)
(field schema: (identifier) @namespace)

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
  (keyword_order_by)
  (keyword_group_by)
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
