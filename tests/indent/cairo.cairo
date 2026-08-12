fn add(a: u64, b: u64) -> u64 {
    let sum = a + b;
    match sum {
        0 => 0,
        _ => sum,
    }
}

struct Point {
    x: u64,
    y: u64,
}
