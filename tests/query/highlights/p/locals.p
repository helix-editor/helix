fun Add(x: int, y: int) : int {
//      ^ @variable.parameter
//              ^ @variable.parameter
//  ^ @function
//  ^ !@variable.parameter
  return x + y;
//       ^ @variable.parameter
//           ^ @variable.parameter
}

machine M {
  start state Init {
    entry (payload: int) {
//         ^ @variable.parameter
      payload = payload + 1;
//    ^ @variable.parameter
//              ^ @variable.parameter
    }
  }
}
