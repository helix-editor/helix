fn f(o Obj) {
  o.do_thing()
//  ^ @function.method
  y := o.field
//       ^ @variable.other.member
}
