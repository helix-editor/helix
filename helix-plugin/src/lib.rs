use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use wasmtime_wasi::tokio::WasiCtxBuilder;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PluginName(String);

impl From<&str> for PluginName {
    fn from(name: &str) -> Self {
        Self(name.to_owned())
    }
}

impl From<String> for PluginName {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl fmt::Display for PluginName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Describes a path accessible from sandbox
#[derive(Debug)]
pub enum DirDef {
    Mirrored {
        path: PathBuf,
    },
    Mapped {
        host_path: PathBuf,
        guest_path: PathBuf,
    },
}

impl DirDef {
    /// Preopens the directory definition for the given WASI context
    pub fn preopen(&self, wasi_builder: WasiCtxBuilder) -> Result<WasiCtxBuilder> {
        use std::fs::File;
        use wasmtime_wasi::Dir;

        let (host_path, guest_path) = match self {
            DirDef::Mirrored { path } => (path.as_path(), path.as_path()),
            DirDef::Mapped {
                host_path,
                guest_path,
            } => (host_path.as_path(), guest_path.as_path()),
        };

        let host_dir = unsafe {
            // SAFETY: user is deciding for himself folders that should be accessibles
            Dir::from_std_file(File::open(host_path)?)
        };

        wasi_builder.preopened_dir(host_dir, guest_path)
    }
}

pub struct HelixCtx {
    wasi: wasmtime_wasi::WasiCtx,
}

pub type HelixStore = wasmtime::Store<HelixCtx>;

pub struct PluginDef {
    pub name: PluginName,
    pub path: PathBuf,
    pub dependencies: Vec<PluginName>,
}

pub struct Plugin {
    instance: wasmtime::Instance,
}

impl Plugin {
    pub fn get_typed_func<Params, Results>(
        &self,
        store: &mut HelixStore,
        name: &str,
    ) -> Result<wasmtime::TypedFunc<Params, Results>>
    where
        Params: wasmtime::WasmParams,
        Results: wasmtime::WasmResults,
    {
        let func = self
            .instance
            .get_typed_func::<Params, Results, _>(store, name)?;
        Ok(func)
    }

    pub fn get_func(&self, store: &mut HelixStore, name: &str) -> Option<wasmtime::Func> {
        self.instance.get_func(store, name)
    }
}

pub struct PluginsSystem {
    pub store: HelixStore,
    pub plugins: HashMap<PluginName, Plugin>,
}

impl PluginsSystem {
    pub fn builder() -> PluginsSystemBuilder {
        PluginsSystemBuilder::default()
    }
}

pub struct PluginsSystemBuilder {
    debug_info: bool,
    definitions: Vec<PluginDef>,
    preopened_dirs: Vec<DirDef>,
    linker_fn: Box<dyn Fn(&mut wasmtime::Linker<HelixCtx>) -> Result<()>>,
}

impl Default for PluginsSystemBuilder {
    fn default() -> Self {
        Self {
            debug_info: false,
            definitions: vec![],
            preopened_dirs: vec![],
            linker_fn: Box::new(|_| Ok(())),
        }
    }
}

impl PluginsSystemBuilder {
    pub fn plugin(&mut self, def: PluginDef) -> &mut Self {
        self.definitions.push(def);
        self
    }

    pub fn plugins(&mut self, mut defs: Vec<PluginDef>) -> &mut Self {
        self.definitions.append(&mut defs);
        self
    }

    pub fn dir(&mut self, dir: DirDef) -> &mut Self {
        self.preopened_dirs.push(dir);
        self
    }

    pub fn dirs(&mut self, mut dirs: Vec<DirDef>) -> &mut Self {
        self.preopened_dirs.append(&mut dirs);
        self
    }

    pub fn debug_info(&mut self, debug: bool) -> &mut Self {
        self.debug_info = debug;
        self
    }

    pub fn linker<F>(&mut self, linker_fn: F) -> &mut Self
    where
        F: Fn(&mut wasmtime::Linker<HelixCtx>) -> Result<()> + 'static,
    {
        self.linker_fn = Box::new(linker_fn);
        self
    }

    /// Instanciate the plugins system, compiling and linking WASM modules as appropriate.
    pub async fn build(&self) -> Result<PluginsSystem> {
        use wasmtime::{Config, Engine, Linker, Module, Store};

        let mut config = Config::new();
        config.debug_info(self.debug_info);
        config.async_support(true);

        let engine = Engine::new(&config)?;

        // Compile plugins
        let modules: HashMap<PluginName, Module> = self
            .definitions
            .iter()
            .map(|def| {
                println!("Compile {}", def.name);
                Module::from_file(&engine, &def.path)
                    .with_context(|| format!("module creation failed for `{}`", def.name))
                    .map(|module| (def.name.clone(), module))
            })
            .collect::<Result<HashMap<PluginName, Module>>>()?;

        // Dumb link order resolution: a good one would detect cycles to give a better error, in
        // our case a link error will arise at link time
        let mut link_order = Vec::new();
        for def in &self.definitions {
            let insert_pos = if let Some(pos) = link_order.iter().position(|name| name == &def.name)
            {
                pos
            } else {
                link_order.push(def.name.clone());
                link_order.len() - 1
            };

            for dep in &def.dependencies {
                if let Some(pos) = link_order.iter().position(|name| name == dep) {
                    if pos > insert_pos {
                        link_order.remove(pos);
                        link_order.insert(insert_pos, dep.clone());
                    }
                } else {
                    link_order.insert(insert_pos, dep.clone());
                };
            }
        }

        // Link and create instances
        let mut wasi_builder = WasiCtxBuilder::new();
        for dir in &self.preopened_dirs {
            wasi_builder = dir
                .preopen(wasi_builder)
                .with_context(|| format!("couldn't preopen directory `{:?}`", dir))?;
        }
        let wasi = wasi_builder.build();

        let ctx = HelixCtx { wasi };

        let mut store = Store::new(&engine, ctx);

        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s: &mut HelixCtx| &mut s.wasi)?;

        (self.linker_fn)(&mut linker).context("couldn't add host-provided modules to linker")?;

        let mut plugins: HashMap<PluginName, Plugin> = HashMap::new();

        for name in link_order {
            let module = modules.get(&name).expect("this module was compiled above");

            let instance = linker
                .instantiate_async(&mut store, module)
                .await
                .with_context(|| format!("couldn't instanciate `{}`", name))?;

            // Register the instance with the linker for the next linking
            linker.instance(&mut store, &name.0, instance)?;

            plugins.insert(name, Plugin { instance });
        }

        // Call `init` function on all loaded plugins
        for plugin in plugins.values_mut() {
            // Call this plugin's `init` function if one is defined
            if let Ok(func) = plugin.get_typed_func::<(), ()>(&mut store, "init") {
                func.call_async(&mut store, ()).await?;
            }
        }

        Ok(PluginsSystem { store, plugins })
    }
}
