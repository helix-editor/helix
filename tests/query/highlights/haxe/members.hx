class C { function g(o) { o.meth(); var y = o.fld; freefn(); } }
//                          ^ @function
//                                            ^ @variable.other.member
//                                                 ^ @function
