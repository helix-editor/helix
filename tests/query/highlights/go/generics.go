package main
func Sum[T Number](xs []T) T {
//       ^ @type.parameter
//         ^ @type
	var f = obj.Method
//           ^ @variable.other.member
	return obj.field
//          ^ @variable.other.member
}
