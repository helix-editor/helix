"""
*bold*
"""

    # TODO
    """
    *bold*
    """

"*bold*"

module A
"""
*bold*
"""
"*bold*"
end

begin
"""
*bold*
"""
"*bold*"
end

@doc """
*bold*
"""

@doc "*bold*"

@doc "*bold*" t = nothing

md"""
*bold*
[text](link)
`code`
# header
"""

# TODO: missing header highlighting
md"*bold* [text](link) `code` # header"

# TODO WARN

#=
TODO
WARN
=#

html"""
<div class="test">test</div>
"""

# TODO: missing highlighting
html"<div class="test">test</div>"

@html_str """
<div class="test">test</div>
"""

# TODO: missing highlighting
@html_str "<div class="test">test</div>"

r"""
[a-z]
"""

r"[a-z]"

`ls -la $(1 + 2)`

gql"""
type A { a: String }
"""

gql"type A { a: String }"

@gql_str """
type A { a: String }
"""

@gql_str "type A { a: String }"

htl"""
<span>$v</span>
"""

htl"<span>$v</span>"

@htl_str """
<span>v</span>
"""

@htl_str "<span>v</span>"

@htl """
<span>v</span>
"""

@htl "<span>v</span>"

L"""
$ 1 / x $
"""

L"$ 1 / x $"

@L_str """
1 + 2
"""

@L_str "1 + 2"

py"""
math.pi / 4
"""

py"math.pi / 4"

@py_str """
math.pi / 4
"""

@py_str "math.pi / 4"

sql```
CREATE TABLE foo (email text, userid integer)
```

sql`CREATE TABLE foo (email text, userid integer)`

@sql_cmd """
CREATE TABLE foo (email text, userid integer)
"""

@sql_cmd "CREATE TABLE foo (email text, userid integer)"

typst"""
$ 1 / x $
"""

typst"$ 1 / x $"

@typst_str """
#let x = 1
"""

@typst_str "#let x = 1"
