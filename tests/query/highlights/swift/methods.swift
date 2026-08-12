func f() {
    invokeit()
//  ^ @function
    obj.doThing()
//      ^ @function
    _ = obj.field
//          ^ @variable.other.member
}
