/// Concatenates strings together.
///
/// `str_concat!(a, " ", b, " ", c)` is:
/// - more performant than `format!("{a} {b} {c}")`
/// - more ergonomic than using `String::with_capacity` followed by a series of `String::push_str`
#[macro_export]
macro_rules! str_concat {
    ($($value:expr),*) => {{
        // Rust does not allow using `+` as separator between value
        // so we must add that at the end of everything. The `0` is necessary
        // at the end so it does not end with "+ " (which would be invalid syntax)
        let mut buf = String::with_capacity($($value.len() + )* 0);
        $(
            buf.push_str(&$value);
        )*
        buf
    }}
}
