struct Point {
//            ^ 4 eof
    x: f32,
    y: f32,
}

fn main() {
//         ^ 4 eof
    if (a) {}
//          ^ 4
    var total = 0.0;
    for (var i = 0; i < 10; i++) {
//                                ^ 8 eof
        if (i > 5) {
//                  ^ 12 eof
            total = total + 1.0;
        }
    }
}
