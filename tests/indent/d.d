import std.stdio;

struct Point {
    int x;
    int y;
}

void main() {
    auto values = [1, 2, 3];
    foreach (v; values) {
        if (v > 1) {
            writeln(v);
        } else {
            writeln("small");
        }
    }
}
