use std::{borrow::Cow, path::PathBuf, sync::Arc};

use dlopen::wrapper::{Container, WrapperApi};
use dlopen_derive::WrapperApi;

use crate::ui::PromptEvent;

use super::{CommandSignature, Context};

// use super::builtin::BuiltInModule;

#[repr(C)]
#[derive(Clone)]
pub struct ExternalModule {
    pub name: Box<str>,
    pub commands: Box<[CrossBoundaryTypableCommand]>,
}

impl ExternalModule {
    pub fn new(name: String, commands: Vec<CrossBoundaryTypableCommand>) -> Self {
        println!("Name: {}", name);

        Self {
            name: name.into_boxed_str(),
            commands: commands.into_boxed_slice(),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

// pub syn_loader: Arc<syntax::Loader>,
// pub theme_loader: Arc<theme::Loader>,

#[repr(C)]
#[derive(Clone)]
pub struct CrossBoundaryTypableCommand {
    pub name: Box<str>,
    pub aliases: Box<[String]>,
    pub doc: Box<str>,
    pub fun: for<'a> extern "C" fn(
        &mut Context<'a>,
        &helix_view::theme::Loader,
        &helix_core::syntax::Loader,
        Box<[Box<str>]>,
        *const PromptEvent,
    ) -> anyhow::Result<()>,
    pub signature: CommandSignature,
}

#[derive(WrapperApi, Clone)]
pub struct ModuleApi {
    generate_module: fn() -> ExternalModule,
}

#[derive(Clone)]
pub(crate) struct DylibContainers {
    pub(crate) containers: Vec<Arc<Container<ModuleApi>>>,
}

impl DylibContainers {
    pub fn new() -> Self {
        Self {
            containers: Vec::new(),
        }
    }

    pub fn load_modules_from_directory(&mut self, home: Option<String>) {
        if let Some(home) = home {
            let mut home = PathBuf::from(home);
            home.push("native");

            if home.exists() {
                let paths = std::fs::read_dir(home).unwrap();

                for path in paths {
                    println!("{:?}", path);

                    let path = path.unwrap().path();

                    if path.extension().unwrap() != "so" && path.extension().unwrap() != "dylib" {
                        continue;
                    }

                    let path_name = path.file_name().and_then(|x| x.to_str()).unwrap();
                    log::info!(target: "dylibs", "Loading dylib: {}", path_name);
                    // Load in the dylib
                    let cont: Container<ModuleApi> = unsafe { Container::load(path) }
                        .expect("Could not open library or load symbols");

                    // Keep the container alive for the duration of the program
                    // This should probably just get wrapped up with the engine as well, when registering modules, directly
                    // register an external dylib
                    self.containers.push(Arc::new(cont));
                }
            } else {
                log::warn!(target: "dylibs", "$STEEL_HOME/native directory does not exist")
            }
        } else {
            log::warn!(target: "dylibs", "STEEL_HOME variable missing - unable to read shared dylibs")
        }
    }

    pub fn create_commands(&self) -> Vec<ExternalModule> {
        self.containers
            .iter()
            .map(|x| x.generate_module())
            .collect()
    }
}
