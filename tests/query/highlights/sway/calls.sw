fn main() {
    obj.doThing();
//      ^ @function
    let x = obj.field;
//              ^ @variable.other.member
    helper(1);
//  ^ @function
}
