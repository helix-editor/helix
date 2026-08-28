interface Shape { size: number; }
//        ^ @type
//                ^ @variable.other.member
//                      ^ @type.builtin
class Box {
//    ^ @type
  method(count: number): void {
//^ @function.method
    obj.doThing();
//      ^ @function.method
  }
}
