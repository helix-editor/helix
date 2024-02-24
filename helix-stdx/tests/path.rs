use std::{
    ffi::OsStr,
    path::{Component, Path},
};

use helix_stdx::path;

#[test]
fn expand_tilde() {
    for path in ["~", "~/foo"] {
        let expanded = path::expand_tilde(Path::new(path));

        let tilde = Component::Normal(OsStr::new("~"));

        let mut component_count = 0;
        for component in expanded.components() {
            // No tilde left.
            assert_ne!(component, tilde);
            component_count += 1;
        }

        // The path was at least expanded to something.
        assert_ne!(component_count, 0);
    }
}
