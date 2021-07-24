use anyhow::{Context, Result};
use libloading::{Library, Symbol};
use tree_sitter::Language;

fn replace_dashes_with_underscores(name: &str) -> String {
    name.replace('-', "_")
}
#[cfg(unix)]
const DYLIB_EXTENSION: &str = "so";

#[cfg(windows)]
const DYLIB_EXTENSION: &str = "dll";

pub fn get_language(runtime_path: &std::path::Path, name: &str) -> Result<Language> {
    let name = name.to_ascii_lowercase();
    let mut library_path = runtime_path.join("grammars").join(&name);
    // TODO: duplicated under build
    library_path.set_extension(DYLIB_EXTENSION);

    let library = unsafe { Library::new(&library_path) }
        .with_context(|| format!("Error opening dynamic library {:?}", &library_path))?;
    let language_fn_name = format!("tree_sitter_{}", replace_dashes_with_underscores(&name));
    let language = unsafe {
        let language_fn: Symbol<unsafe extern "C" fn() -> Language> = library
            .get(language_fn_name.as_bytes())
            .with_context(|| format!("Failed to load symbol {}", language_fn_name))?;
        language_fn()
    };
    std::mem::forget(library);
    Ok(language)
}
