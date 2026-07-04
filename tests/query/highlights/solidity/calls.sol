contract C {
  function f() public {
    obj.doThing();
//      ^ @function.method
    helper(1);
//  ^ @function
    uint x = obj.field;
//               ^ @variable.other.member
  }
}
