use anyhow::Result;
use helix_loader::grammar::fetch_grammars;

// This binary is used in the Release CI as an optimization to cut down on
// compilation time. This is not meant to be run manually.

fn main() -> Result<()> {
    fetch_grammars()
}
