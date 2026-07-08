fn run() {
// ^ @function
    let y = bar(2);
//          ^ @function
    let z = obj.field;
//              ^ @variable.other.member
}
