function add(a: number): number {
//       ^ @function
//           ^ @variable.parameter
//              ^ @type.builtin
  return a + 1;
}
const el = <Foo bar={add(2)}>hi</Foo>;
//          ^ @constructor
//              ^ @attribute
//                   ^ @function
//                               ^ @constructor
