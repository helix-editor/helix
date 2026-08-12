fn m() {
    obj.first().second();
//      ^ @function.method
//               ^ @function.method
    let n = outer(inner(x));
//          ^ @function
//                ^ @function
    let f = build().field;
//                  ^ @variable.other.member
}
