function! F(o)
  call G(a:o)
"      ^ @function
  let x = a:o.prop
"             ^ @variable.other.member
endfunction
