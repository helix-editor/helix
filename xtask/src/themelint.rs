use crate::paths;
use crate::DynError;
use helix_view::theme::Style;
use helix_view::Theme;

pub fn lint_all() -> Result<(), DynError> {
    let files = std::fs::read_dir(paths::themes())
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_dir() {
                None
            } else {
                Some(path.file_name()?.to_string_lossy().to_string())
            }
        })
        .collect::<Vec<String>>();
    let mut errors = vec![];
    files
        .into_iter()
        .for_each(|path| match lint(path.replace(".toml", "")) {
            Err(err) => {
                let errs: String = err.to_string();
                errors.push(errs)
            }
            _ => return,
        });
    if errors.len() > 0 {
        Err(errors.join(" ").into())
    } else {
        Ok(())
    }
}

pub fn lint(file: String) -> Result<(), DynError> {
    let path = paths::themes().join(file + ".toml");
    let theme = std::fs::read(&path).unwrap();
    let theme: Theme = toml::from_slice(&theme).expect("Failed to parse theme");
    let check = vec![
        "ui.background",
        "ui.virtual",
        "ui.cursor",
        "ui.selection",
        "ui.linenr",
        "ui.text",
        "ui.popup",
        "ui.window",
        "ui.menu",
        "ui.statusline",
        "ui.cursorline.primary",
    ];

    let lint_errors: Vec<String> = check
        .into_iter()
        .filter_map(|path| {
            let style = theme.get(path);
            if style.eq(&Style::default()) {
                Some(path.split(".").take(2).collect::<Vec<&str>>().join("."))
            } else {
                None
            }
        })
        .collect();

    if lint_errors.len() > 0 {
        println!("{:?}", path);
        println!("{:?}", lint_errors);
    }

    Ok(())
}
