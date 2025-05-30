use helix_loader::grammar::{build_grammars, fetch_grammars};

fn main() {
    if std::env::var("HELIX_DISABLE_AUTO_GRAMMAR_BUILD").is_err() {
        fetch_grammars().expect("Failed to fetch tree-sitter grammars");
        build_grammars(Some(std::env::var("TARGET").unwrap()))
            .expect("Failed to compile tree-sitter grammars");
    }

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
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

        let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap();
        match target_env.as_str() {
            "msvc" => {
                compile_with_toolkit_msvc(&output_dir, rc_path);

                println!("cargo:rustc-link-search=native={}", output_dir.display());
                println!("cargo:rustc-link-lib=dylib=resource");
            }
            "gnu" => {
                compile_with_toolkit_gnu(&output_dir, rc_path);

                println!("cargo:rustc-link-search=native={}", output_dir.display());
                println!("cargo:rustc-link-lib=static:+whole-archive=resource");
            }
            _ => panic!("Can only compile resource file when target_env is \"gnu\" or \"msvc\""),
        }
    }

    fn compile_with_toolkit_msvc(output: &Path, input: PathBuf) {
        let rc_exe = find_rc_exe().expect("Windows SDK is to be installed along with MSVC");

        let mut command = process::Command::new(rc_exe);
        let command = command.arg(format!(
            "/I{}",
            env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR should have been set by Cargo")
        ));

        let lib_path = PathBuf::from(&output).join("resource.lib");
        let status = command
            .arg(format!("/fo{}", lib_path.display()))
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

    fn compile_with_toolkit_gnu(output: &PathBuf, input: PathBuf) {
        let windres_exe = find_windres_exe();
        let ar_exe = find_ar_exe();

        let mut command = process::Command::new(windres_exe);
        let command = command.arg(format!(
            "-I{}",
            env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR should have been set by Cargo")
        ));

        let obj_path = PathBuf::from(&output).join("resource.o");
        let status = command
            .arg(format!("{}", input.display()))
            .arg(format!("{}", obj_path.display()))
            .output()
            .unwrap();

        println!(
            "WINDRES Output:\n{}\n------",
            String::from_utf8_lossy(&status.stdout)
        );
        println!(
            "WINDRES Error:\n{}\n------",
            String::from_utf8_lossy(&status.stderr)
        );

        let mut command = process::Command::new(ar_exe);

        let lib_path = PathBuf::from(&output).join("libresource.a");
        let status = command
            .arg("rsc")
            .arg(format!("{}", lib_path.display()))
            .arg(format!("{}", obj_path.display()))
            .output()
            .unwrap();

        println!(
            "AR Output:\n{}\n------",
            String::from_utf8_lossy(&status.stdout)
        );
        println!(
            "AR Error:\n{}\n------",
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

    fn find_prefix() -> String {
        // This code snippet is peeked from crate `winresource`.
        if let Ok(cross) = env::var("CROSS_COMPILE") {
            cross
        } else if env::var_os("HOST").unwrap() != env::var_os("TARGET").unwrap()
            && cfg!(not(all(windows, target_env = "msvc")))
        // use mingw32 under linux
        {
            match env::var("TARGET").unwrap().as_str() {
                        "x86_64-pc-windows-msvc" | // use mingw32 under linux
                        "x86_64-pc-windows-gnu" => "x86_64-w64-mingw32-",
                        "i686-pc-windows-msvc" | // use mingw32 under linux
                        "i686-pc-windows-gnu" => "i686-w64-mingw32-",
                        // MinGW supports ARM64 only with an LLVM-based toolchain
                        // (x86 users might also be using LLVM, but we can't tell that from the Rust target...)
                        "aarch64-pc-windows-gnu" => "llvm-",
                        // *-gnullvm targets by definition use LLVM-based toolchains
                        "x86_64-pc-windows-gnullvm"
                        | "i686-pc-windows-gnullvm"
                        | "aarch64-pc-windows-gnullvm" => "llvm-",
                        // fail safe
                        target => {
                            println!(
                                "cargo:warning=unknown Windows target {target} used for cross-compilation; \
                                      invoking unprefixed windres"
                            );
                            ""
                        }
                    }
                    .into()
        } else {
            "".into()
        }
    }

    fn find_windres_exe() -> PathBuf {
        match env::var("WINDRES") {
            Ok(windres) => windres.into(),
            Err(_) => format!("{}windres", find_prefix()).into(),
        }
    }

    fn find_ar_exe() -> PathBuf {
        match env::var("AR") {
            Ok(ar) => ar.into(),
            Err(_) => format!("{}ar", find_prefix()).into(),
        }
    }

    fn write_resource_file(rc_path: &Path, icon_path: &str) -> io::Result<()> {
        let mut f = std::fs::File::create(rc_path)?;
        writeln!(f, "{} ICON \"{}\"", 1, icon_path)?;

        Ok(())
    }
}
