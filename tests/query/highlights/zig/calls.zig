const std = @import("std");
//          ^ @keyword.control.import
pub fn main() void {
    obj.doThing();
//      ^ @function.method
    helper(x);
//  ^ @function
    const y = obj.field;
//                ^ @variable.other.member
}
