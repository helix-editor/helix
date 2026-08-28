struct Point {
    x: f32,
    y: f32,
}

fn main() {
    var total = 0.0;
    for (var i = 0; i < 10; i++) {
        if (i > 5) {
            total = total + 1.0;
        }
    }
}
