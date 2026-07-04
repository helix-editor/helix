module a::m {
    struct S has copy {
        x: u64,
        y: u64,
    }

    fun f(o: S): u64 {
        let total = 0;
        if (o.x > 0) {
            total = o.x;
        } else {
            total = o.y;
        }
        while (total > 0) {
            total = total - 1;
        }
        total
    }

    fun classify(n: u64): u64 {
        match (n) {
            0 => 1,
            _ => 0,
        }
    }
}
