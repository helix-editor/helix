package main
//      ^ @namespace
type Point struct { X int }
//   ^ @type.definition
//                  ^ @variable.other.member
//                    ^ @type.builtin
func (p Point) Dist(count int) bool {
//             ^ @function.method
//                  ^ @variable.parameter
    return helper(count)
//         ^ @function
}
