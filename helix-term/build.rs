use helix_loader::grammar::{build_grammars, fetch_grammars};

const STRICT: bool = true;

fn main() {
    if std::env::var("HELIX_DISABLE_AUTO_GRAMMAR_BUILD").is_err() {
        fetch_grammars(STRICT).expect("Failed to fetch tree-sitter grammars");
        build_grammars(Some(std::env::var("TARGET").unwrap()), STRICT)
            .expect("Failed to compile tree-sitter grammars");
    }

    // NOTE: #[cfg(windows)] would reflect the host compiling this build
    // script, not the target. Query CARGO_CFG_TARGET_OS so cross-compiling to
    // Windows (e.g. cargo build --target x86_64-pc-windows-gnu on Linux) also
    // embeds the icon.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        windows_rc::link_icon_in_windows_exe("../contrib/helix-256p.ico");
    }
}

mod windows_rc {
    use std::io::prelude::Write;
    use std::{env, io, path::Path, path::PathBuf, process};

    pub(crate) fn link_icon_in_windows_exe(icon_path: &str) {
        let output = env::var("OUT_DIR").expect("Env var OUT_DIR should have been set by compiler");
        let output_dir = PathBuf::from(output);

        let rc_path = output_dir.join("resource.rc");
        write_resource_file(&rc_path, icon_path).unwrap();

        let resource_file = PathBuf::from(&output_dir).join("resource.lib");

        // CARGO_CFG_TARGET_ENV describes the target, unlike cfg!(target_env)
        // which describes the host running this build script.
        let compiled = match env::var("CARGO_CFG_TARGET_ENV").as_deref() {
            Ok("msvc") => match find_rc_exe() {
                Ok(rc_exe) => {
                    compile_with_toolkit_msvc(rc_exe, &resource_file, &rc_path);
                    true
                }
                Err(e) => {
                    // Gracefully continue without an icon, e.g. when
                    // cross-compiling to windows-msvc from a host with no
                    // Windows SDK. Mirrors the previous behavior where the
                    // resource step was skipped entirely on non-Windows hosts.
                    println!(
                        "cargo:warning=could not locate an rc-compatible compiler ({e}); \
                         hx.exe will be built without an application icon. Set $RC to \
                         one (e.g. rc.exe or llvm-rc) to embed it."
                    );
                    false
                }
            },
            Ok("gnu") => {
                let windres = find_windres().expect(
                    "windres is required for windows-gnu targets;                      install binutils/mingw-w64 or set $WINDRES",
                );
                compile_with_windres_gnu(windres, &resource_file, &rc_path);
                true
            }
            other => panic!("unsupported Windows target environment: {:?}", other),
        };

        if compiled {
            println!("cargo:rustc-link-search=native={}", output_dir.display());
            println!("cargo:rustc-link-lib=dylib=resource");
        }
    }

    /// Locate a windres binary, honoring `$WINDRES`, the binutils
    /// target-prefixed name used by cross toolchains (e.g.
    /// `x86_64-w64-mingw32-windres` for `x86_64-pc-windows-gnu`), and finally
    /// a bare `windres` in PATH as found on MSYS2/MinGW installs.
    fn find_windres() -> Option<PathBuf> {
        let mut candidates: Vec<String> = env::var("WINDRES").into_iter().collect();
        if let Some(prefix) = env::var("TARGET").ok().as_deref().and_then(gnu_tool_prefix) {
            candidates.push(format!("{}windres", prefix));
        }
        candidates.push("windres".into());
        candidates.push("windres.exe".into());
        candidates
            .into_iter()
            .map(PathBuf::from)
            .find(check_if_exe_works)
    }

    /// Map a Rust target triple to the binutils tool prefix used by mingw-w64
    /// cross toolchains.
    fn gnu_tool_prefix(target: &str) -> Option<&'static str> {
        match target {
            "x86_64-pc-windows-gnu" => Some("x86_64-w64-mingw32-"),
            "i686-pc-windows-gnu" => Some("i686-w64-mingw32-"),
            "aarch64-pc-windows-gnu" => Some("aarch64-w64-mingw32-"),
            _ => None,
        }
    }

    fn check_if_exe_works(exe: &PathBuf) -> bool {
        process::Command::new(exe)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn compile_with_windres_gnu(windres: PathBuf, output: &PathBuf, input: &PathBuf) {
        let mut command = process::Command::new(windres);
        let command = command.arg(format!(
            "--include-dir={}",
            env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR should have been set by Cargo")
        ));

        let status = command
            .arg(format!("--output={}", output.display()))
            .arg(format!("{}", input.display()))
            .output()
            .unwrap();

        println!(
            "Windres Output:\n{}\n------",
            String::from_utf8_lossy(&status.stdout)
        );
        println!(
            "Windres Error:\n{}\n------",
            String::from_utf8_lossy(&status.stderr)
        );
    }

    fn compile_with_toolkit_msvc(rc_exe: PathBuf, output: &PathBuf, input: &PathBuf) {
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
        // An explicit $RC override (e.g. llvm-rc for cross builds) wins over
        // the Windows SDK registry lookup.
        if let Some(rc) = env::var("RC").ok().map(PathBuf::from) {
            if check_if_exe_works(&rc) {
                return Ok(rc);
            }
        }
        let find_reg_key = process::Command::new("reg")
            .arg("query")
            .arg(r"HKLM\SOFTWARE\Microsoft\Windows Kits\Installed Roots")
            .arg("/reg:32")
            .arg("/v")
            .arg("KitsRoot10")
            .output();

        match find_reg_key {
            Err(find_reg_key) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to run registry query: {}", find_reg_key),
            )),
            Ok(find_reg_key) => {
                if find_reg_key.status.code().unwrap() != 0 {
                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Can not find Windows SDK",
                    ))
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
