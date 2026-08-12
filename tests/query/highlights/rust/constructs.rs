use std::fmt;
//  ^ @namespace
enum Color { Red }
//           ^ @type.enum.variant
fn f(self) {
//   ^ @variable.builtin
    println!("ok");
//  ^ @function.macro
    self.helper();
//       ^ @function.method
    let p = Point {};
//          ^ @constructor
}
