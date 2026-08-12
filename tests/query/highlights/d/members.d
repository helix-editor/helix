void f(Obj o) {
    auto x = o.field;
//             ^ @variable.other.member
    o.method(1);
//    ^ @function
}
