use anyhow::Result;
use helix_loader::grammar::{bundled_config, fetch};

// This binary is used in the Release CI as an optimization to cut down on
// compilation time. This is not meant to be run manually.

const STRICT: bool = true;

fn main() -> Result<()> {
    fetch(bundled_config(), STRICT)
}
