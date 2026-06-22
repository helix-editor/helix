proc f(o: Obj) =
  let v = o.field
  #         ^ @variable.other.member
  echo o.meth(1)
  #      ^ @function
