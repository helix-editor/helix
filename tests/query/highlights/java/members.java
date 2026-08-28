class C { void g(O o) { o.meth(); int y = o.fld; int z = Color.RED; } }
//                        ^ @function.method
//                                          ^ @variable.other.member
//                                                             ^ @constant
