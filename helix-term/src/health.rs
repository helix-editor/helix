pub fn general() {
    use crossterm::style::Stylize;

    let config_file = helix_core::config_file();
    let lang_file = helix_core::lang_config_file();
    let log_file = helix_core::log_file();
    let rt_dir = helix_core::runtime_dir();

    if config_file.exists() {
        println!("Config file: {}", config_file.display());
    } else {
        println!("Config file: default")
    }
    if lang_file.exists() {
        println!("Language file: {}", lang_file.display());
    } else {
        println!("Language file: default")
    }
    println!("Log file: {}", log_file.display());
    println!("Runtime directory: {}", rt_dir.display());

    if let Ok(path) = std::fs::read_link(&rt_dir) {
        let msg = format!("Runtime directory is symlinked to {}", path.display());
        println!("{}", msg.yellow());
    }
    if !rt_dir.exists() {
        println!("{}", "Runtime directory does not exist.".red());
    }
    if rt_dir.read_dir().ok().map(|it| it.count()) == Some(0) {
        println!("{}", "Runtime directory is empty.".red());
    }
}
