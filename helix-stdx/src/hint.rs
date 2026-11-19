/// Hints to the compiler that current code path is cold.
#[cold]
pub const fn cold_path() {}

/// Hints to the compiler that branch condition is likely to be true.
///
/// Returns the value passed to it.
///
/// Any use other than with `if` statements will probably not have an effect.
#[inline(always)]
pub const fn likely(b: bool) -> bool {
    if b {
        true
    } else {
        cold_path();
        false
    }
}

/// Hints to the compiler that branch condition is likely to be false.
///
/// Returns the value passed to it.
///
/// Any use other than with `if` statements will probably not have an effect.
#[inline(always)]
pub const fn unlikely(b: bool) -> bool {
    if b {
        cold_path();
        true
    } else {
        false
    }
}
