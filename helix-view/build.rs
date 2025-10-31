fn main() {
    // alias scancode feature flag
    println!("cargo::rustc-check-cfg=cfg(scancode)");
    #[cfg(any(
        feature = "scancode-query",
        feature = "scancode-evdev",
        feature = "scancode-hidapi"
    ))]
    println!("cargo:rustc-cfg=scancode")
}
