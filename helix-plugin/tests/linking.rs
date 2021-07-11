use helix_plugin::{DirDef, PluginDef, PluginName, PluginsSystem};
use std::path::PathBuf;

/// Based on https://docs.wasmtime.dev/examples-rust-linking.html
#[tokio_macros::test]
async fn two_plugins() {
    let linking1 = PluginName::from("linking1");
    let linking2 = PluginName::from("linking2");

    let mut system = PluginsSystem::builder()
        .plugin(PluginDef {
            name: linking1.clone(),
            path: PathBuf::from("tests/linking1.wat"),
            dependencies: vec![linking2.clone()],
        })
        .plugin(PluginDef {
            name: linking2.clone(),
            path: PathBuf::from("tests/linking2.wat"),
            dependencies: vec![],
        })
        .build()
        .await
        .unwrap();

    let run = system
        .plugins
        .get(&linking1)
        .unwrap()
        .get_typed_func::<(), ()>(&mut system.store, "run")
        .unwrap();
    run.call_async(&mut system.store, ()).await.unwrap();

    let double = system
        .plugins
        .get(&linking2)
        .unwrap()
        .get_typed_func::<i32, i32>(&mut system.store, "double")
        .unwrap();
    assert_eq!(double.call_async(&mut system.store, 5).await.unwrap(), 10);
}

#[tokio_macros::test(flavor = "multi_thread")]
async fn wasi_preopen_dir() {
    let name = PluginName::from("read_file");

    // read_file.wasm is a program reading `./Cargo.toml` file and does nothing with it

    PluginsSystem::builder()
        .dir(DirDef::Mirrored {
            path: PathBuf::from("./"),
        })
        .plugin(PluginDef {
            name: name.clone(),
            path: PathBuf::from("tests/read_file.wasm"),
            dependencies: vec![],
        })
        .build()
        .await
        .unwrap();
}

#[tokio_macros::test]
async fn callback_to_host() {
    use std::sync::atomic::{AtomicBool, Ordering};

    static CANARY: AtomicBool = AtomicBool::new(false);

    PluginsSystem::builder()
        .plugin(PluginDef {
            name: PluginName::from("call_host"),
            path: PathBuf::from("tests/call_host.wat"),
            dependencies: vec![],
        })
        .linker(|l| {
            l.func_wrap("host", "callback", || CANARY.store(true, Ordering::Relaxed))?;
            Ok(())
        })
        .build()
        .await
        .unwrap();

    assert_eq!(CANARY.load(Ordering::Relaxed), true);
}

#[tokio_macros::test]
async fn callback_to_content() {
    todo!()
}
