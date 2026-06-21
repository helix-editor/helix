const std = @import("std");

const Point = struct {
    x: i32,
    y: i32,
};

const Color = enum {
    red,
    green,
};

const Value = union(enum) {
    int: i32,
    float: f64,
};

fn classify(x: u32) u32 {
    const result = switch (x) {
        0 => 1,
        1, 2 => 2,
        else => 0,
    };
    var i: usize = 0;
    while (i < 10) : (i += 1) {
        if (i > 5) {
            process(i);
        } else {
            other(i);
        }
    }
    const data = [_]u32{
        1,
        2,
    };
    const point = .{
        .x = 1,
        .y = 2,
    };
    return result;
}
