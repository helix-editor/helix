use helix_loader::grammar::{build_grammars, fetch_grammars};

fn main() {
    if std::env::var("HELIX_DISABLE_AUTO_GRAMMAR_BUILD").is_err() {
        fetch_grammars().expect("Failed to fetch tree-sitter grammars");
        build_grammars(Some(std::env::var("TARGET").unwrap()))
            .expect("Failed to compile tree-sitter grammars");
    }

    #[cfg(windows)]
    windows_rc::link_icon_in_windows_exe("../contrib/helix-256p.ico");
}

#[cfg(windows)]
mod windows_rc {
    use std::io::prelude::Write;
    use std::{env, io, path::Path, path::PathBuf, process};

    pub(crate) fn link_icon_in_windows_exe(icon_path: &str) {
        let rc_exe = find_rc_exe().expect("Windows SDK is to be installed along with MSVC");

        let output = env::var("OUT_DIR").expect("Env var OUT_DIR should have been set by compiler");
        let output_dir = PathBuf::from(output);

        let rc_path = output_dir.join("resource.rc");
        write_resource_file(&rc_path, icon_path).unwrap();

        let resource_file = PathBuf::from(&output_dir).join("resource.lib");
        compile_with_toolkit_msvc(rc_exe, resource_file, rc_path);

        println!("cargo:rustc-link-search=native={}", output_dir.display());
        println!("cargo:rustc-link-lib=dylib=resource");
    }

    fn compile_with_toolkit_msvc(rc_exe: PathBuf, output: PathBuf, input: PathBuf) {
        let mut command = process::Command::new(rc_exe);
        let command = command.arg(format!(
            "/I{}",
            env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR should have been set by Cargo")
        ));

        let status = command
            .arg(format!("/fo{}", output.display()))
            .arg(format!("{}", input.display()))
            .output()
            .unwrap();

        println!(
            "RC Output:\n{}\n------",
            String::from_utf8_lossy(&status.stdout)
        );
        println!(
            "RC Error:\n{}\n------",
            String::from_utf8_lossy(&status.stderr)
        );
    }

    fn find_rc_exe() -> io::Result<PathBuf> {
        let find_reg_key = process::Command::new("reg")
            .arg("query")
            .arg(r"HKLM\SOFTWARE\Microsoft\Windows Kits\Installed Roots")
            .arg("/reg:32")
            .arg("/v")
            .arg("KitsRoot10")
            .output();

        match find_reg_key {
            Err(find_reg_key) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to run registry query: {}", find_reg_key),
                ))
            }
            Ok(find_reg_key) => {
                if find_reg_key.status.code().unwrap() != 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Can not find Windows SDK",
                    ));
                } else {
                    let lines = String::from_utf8(find_reg_key.stdout)
                        .expect("Should be able to parse the output");
                    let mut lines: Vec<&str> = lines.lines().collect();
                    let mut rc_exe_paths: Vec<PathBuf> = Vec::new();
                    lines.reverse();
                    for line in lines {
                        if line.trim().starts_with("KitsRoot") {
                            let kit: String = line
                                .chars()
                                .skip(line.find("REG_SZ").unwrap() + 6)
                                .skip_while(|c| c.is_whitespace())
                                .collect();

                            let p = PathBuf::from(&kit);
                            let rc = if cfg!(target_arch = "x86_64") {
                                p.join(r"bin\x64\rc.exe")
                            } else {
                                p.join(r"bin\x86\rc.exe")
                            };

                            if rc.exists() {
                                println!("{:?}", rc);
                                rc_exe_paths.push(rc.to_owned());
                            }

                            if let Ok(bin) = p.join("bin").read_dir() {
                                for e in bin.filter_map(|e| e.ok()) {
                                    let p = if cfg!(target_arch = "x86_64") {
                                        e.path().join(r"x64\rc.exe")
                                    } else {
                                        e.path().join(r"x86\rc.exe")
                                    };
                                    if p.exists() {
                                        println!("{:?}", p);
                                        rc_exe_paths.push(p.to_owned());
                                    }
                                }
                            }
                        }
                    }
                    if rc_exe_paths.is_empty() {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "Can not find Windows SDK",
                        ));
                    }

                    println!("{:?}", rc_exe_paths);
                    let rc_path = rc_exe_paths.pop().unwrap();

                    let rc_exe = if !rc_path.exists() {
                        if cfg!(target_arch = "x86_64") {
                            PathBuf::from(rc_path.parent().unwrap()).join(r"bin\x64\rc.exe")
                        } else {
                            PathBuf::from(rc_path.parent().unwrap()).join(r"bin\x86\rc.exe")
                        }
                    } else {
                        rc_path
                    };

                    println!("Selected RC path: '{}'", rc_exe.display());
                    Ok(rc_exe)
                }
            }
        }
    }

    fn write_resource_file(rc_path: &Path, icon_path: &str) -> io::Result<()> {
        let mut f = std::fs::File::create(rc_path)?;
        writeln!(f, "{} ICON \"{}\"", 1, icon_path)?;

        Ok(())
    }
}
