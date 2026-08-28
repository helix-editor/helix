class C { void G(O o){ o.meth(); int y = o.fld; freefn(); } }
//                       ^ @function
//                                         ^ @variable.other.member
//                                              ^ @function
