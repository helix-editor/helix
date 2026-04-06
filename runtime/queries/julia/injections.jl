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

L"""
$ 1 / x $
"""

L"$ 1 / x $"

r"""
[a-z]
"""

r"[a-z]"

`ls -la $(1 + 2)`

typst"""
$ 1 / x $
"""

typst"$ 1 / x $"
