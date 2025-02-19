fn main() {
    // alias scancode feature flag
    #[cfg(any(feature = "scancode-query", feature = "scancode-evdev", feature = "scancode-hidapi"))]
    println!("cargo:rustc-cfg=scancode")
}
