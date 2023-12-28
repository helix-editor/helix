use helix_loader::command_histfile;
use std::{fs::OpenOptions, io::Write};

pub fn push_history(register: char, line: &str) {
    let filepath = match register {
        ':' => command_histfile(),
        _ => return,
    };

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(filepath)
        // TODO: do something about this unwrap
        .unwrap();

    // TODO: do something about this unwrap
    writeln!(file, "{}", line).unwrap();
}
