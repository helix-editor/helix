fn m() {
    let r = a.b.c().d.e();
//            ^ @variable.other.member
//              ^ @function.method
//                  ^ @variable.other.member
//                    ^ @function.method
}
