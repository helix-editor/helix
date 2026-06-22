void g(Obj o) { o.meth(); var a = o.fld; var b = Foo.BAR; }
//                ^ @function
//                                  ^ @variable.other.member
//                                                   ^ @constant
