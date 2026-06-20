fn f() {
    let y = bar(2);
//          ^ @function
    let z = obj.field;
//              ^ @variable.other.member
    let w = abs(z);
//          ^ @function.builtin
}
