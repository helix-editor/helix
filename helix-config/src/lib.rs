use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};

use anyhow::bail;
use hashbrown::hash_map::Entry;
use hashbrown::HashMap;
use indexmap::IndexMap;

use any::ConfigData;
use convert::ty_into_value;
pub use convert::IntoTy;
pub use definition::{init_config, init_language_server_config};
pub use store::{ConfigStore, DocumentId, LayerId, ScopeId, LanguageId};
pub use toml::read_toml_config;
use validator::StaticValidator;
pub use validator::{regex_str_validator, ty_validator, IntegerRangeValidator, Ty, Validator};
pub use value::{from_value, to_value, Value};

mod any;
mod convert;
mod core_impls;
pub mod definition;
pub mod env;
mod macros;
pub mod store;
mod toml;
mod validator;
mod value;

pub type Guard<'a, T> = MappedRwLockReadGuard<'a, T>;
pub type Map<T> = IndexMap<Box<str>, T, ahash::RandomState>;
pub type String = Box<str>;
pub type List<T> = Box<[T]>;

/// Normalizes an option name by converting hyphens to underscores.
/// This allows users to use either `line-number` or `line_number` in config files.
fn normalize_name(name: &str) -> std::borrow::Cow<'_, str> {
    if name.contains('-') {
        std::borrow::Cow::Owned(name.replace('-', "_"))
    } else {
        std::borrow::Cow::Borrowed(name)
    }
}

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct OptionInfo {
    pub name: Arc<str>,
    pub description: Box<str>,
    pub validator: Box<dyn Validator>,
    pub into_value: fn(&ConfigData) -> Value,
}

pub struct OptionManager {
    vals: RwLock<HashMap<Arc<str>, ConfigData>>,
    parent: Option<Arc<OptionManager>>,
}

impl std::fmt::Debug for OptionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OptionManager")
            .field("vals", &"<RwLock>")
            .field("parent", &self.parent.as_ref().map(|_| "..."))
            .finish()
    }
}

impl OptionManager {
    /// Gets a reference to a config value, looking up the parent chain if not found.
    /// Returns a guard that holds the read lock.
    pub fn get<T: Any>(&self, option: &str) -> Guard<'_, T> {
        Guard::map(self.get_data(option), ConfigData::get)
    }

    /// Gets the raw ConfigData for an option, looking up the parent chain.
    pub fn get_data(&self, option: &str) -> Guard<'_, ConfigData> {
        let option = normalize_name(option);
        let mut current_scope = self;
        loop {
            let lock = current_scope.vals.read();
            if let Ok(res) = RwLockReadGuard::try_map(lock, |options| options.get(option.as_ref()))
            {
                return res;
            }
            let Some(new_scope) = current_scope.parent.as_deref() else {
                unreachable!("option must be at least defined in the global scope")
            };
            current_scope = new_scope;
        }
    }

    /// Gets a dereferenced config value. Useful for String -> &str, List<T> -> &[T].
    pub fn get_deref<T: Deref + Any>(&self, option: &str) -> Guard<'_, T::Target> {
        Guard::map(self.get::<T>(option), T::deref)
    }

    /// Gets a cloned config value, looking up the parent chain if not found.
    pub fn get_cloned<T: Any + Clone>(&self, option: &str) -> T {
        let option = normalize_name(option);
        let mut current_scope = self;
        loop {
            let guard = current_scope.vals.read();
            if let Some(data) = guard.get(option.as_ref()) {
                return data.get::<T>().clone();
            }
            drop(guard);
            let Some(new_scope) = current_scope.parent.as_deref() else {
                unreachable!("option must be at least defined in the global scope")
            };
            current_scope = new_scope;
        }
    }

    pub fn get_folded<T: Any, R>(
        &self,
        option: &str,
        init: R,
        mut fold: impl FnMut(&T, R) -> R,
    ) -> R {
        let option = normalize_name(option);
        let mut res = init;
        let mut current_scope = self;
        loop {
            let guard = current_scope.vals.read();
            if let Some(val) = guard.get(option.as_ref()) {
                let opt_val: &T = val.get();
                res = fold(opt_val, res);
            }
            drop(guard);
            let Some(new_scope) = current_scope.parent.as_deref() else {
                break
            };
            current_scope = new_scope;
        }
        res
    }

    pub fn get_value(
        &self,
        option: impl Into<Arc<str>>,
        registry: &OptionRegistry,
    ) -> anyhow::Result<Value> {
        let option: Arc<str> = option.into();
        let Some(opt) = registry.get(&option) else { bail!("unknown option {option:?}") };
        let data = self.get_data(&option);
        Ok((opt.into_value)(&data))
    }

    pub fn create_scope(self: &Arc<OptionManager>) -> OptionManager {
        OptionManager {
            vals: RwLock::new(HashMap::new()),
            parent: Some(self.clone()),
        }
    }

    pub fn set_parent_scope(&mut self, parent: Arc<OptionManager>) {
        self.parent = Some(parent)
    }

    pub fn set_unchecked(&self, option: Arc<str>, val: ConfigData) {
        let mut guard = self.vals.write();
        guard.insert(option, val);
    }

    pub fn append(
        &self,
        option: impl Into<Arc<str>>,
        val: impl Into<Value>,
        registry: &OptionRegistry,
        max_depth: usize,
    ) -> anyhow::Result<()> {
        let val = val.into();
        let option: Arc<str> = normalize_name(&option.into()).into_owned().into();
        let Some(opt) = registry.get(&option) else { bail!("unknown option {option:?}") };
        // Get old value by traversing parent chain
        let mut old = self.get_value(option.clone(), registry)?;
        old.append(val, max_depth);
        let val = opt.validator.validate(old)?;
        self.set_unchecked(option, val);
        Ok(())
    }

    /// Sets the value of a config option. Returns an error if this config
    /// option doesn't exist or the provided value is not valid.
    pub fn set(
        &self,
        option: impl Into<Arc<str>>,
        val: impl Into<Value>,
        registry: &OptionRegistry,
    ) -> anyhow::Result<()> {
        let option: Arc<str> = normalize_name(&option.into()).into_owned().into();
        let val = val.into();
        let Some(opt) = registry.get(&option) else { bail!("unknown option {option:?}") };
        let val = opt.validator.validate(val)?;
        self.set_unchecked(option, val);
        Ok(())
    }

    /// unsets an options so that its value will be read from
    /// the parent scope instead
    pub fn unset(&self, option: &str) {
        let option = normalize_name(option);
        let mut guard = self.vals.write();
        guard.remove(option.as_ref());
    }
}

#[derive(Debug)]
pub struct OptionRegistry {
    options: HashMap<Arc<str>, OptionInfo>,
    defaults: Arc<OptionManager>,
}

impl OptionRegistry {
    pub fn new() -> Self {
        Self {
            options: HashMap::with_capacity(1024),
            defaults: Arc::new(OptionManager {
                vals: RwLock::new(HashMap::with_capacity(1024)),
                parent: None,
            }),
        }
    }

    pub fn register<T: IntoTy>(&mut self, name: &str, description: &str, default: T) {
        self.register_with_validator(
            name,
            description,
            default,
            StaticValidator::<T::Ty> { ty: PhantomData },
        );
    }

    pub fn register_with_validator<T: IntoTy>(
        &mut self,
        name: &str,
        description: &str,
        default: T,
        validator: impl Validator,
    ) {
        let mut name: Arc<str> = name.into();
        // convert from snake case to kebab case in place without an additional
        // allocation this is save since we only replace ascii with ascii in
        // place std really ougth to have a function for this :/
        // TODO: move to stdx as extension trait
        for byte in unsafe { Arc::get_mut(&mut name).unwrap().as_bytes_mut() } {
            if *byte == b'-' {
                *byte = b'_';
            }
        }
        let default = default.into_ty();
        match self.options.entry(name.clone()) {
            Entry::Vacant(e) => {
                // make sure the validator is correct
                if cfg!(debug_assertions) {
                    validator.validate(T::Ty::to_value(&default))
                        .unwrap_or_else(|err| panic!("Failed to validate default value for option '{}': {}", name, err));
                }
                let opt = OptionInfo {
                    name: name.clone(),
                    description: description.into(),
                    validator: Box::new(validator),
                    into_value: ty_into_value::<T::Ty>,
                };
                e.insert(opt);
            }
            Entry::Occupied(ent) => {
                ent.get()
                    .validator
                    .validate(T::Ty::to_value(&default))
                    .unwrap();
            }
        }
        self.defaults.set_unchecked(name, ConfigData::new(default));
    }

    pub fn global_scope(&self) -> Arc<OptionManager> {
        self.defaults.clone()
    }

    pub fn get(&self, name: &str) -> Option<&OptionInfo> {
        let name = normalize_name(name);
        self.options.get(name.as_ref())
    }
}

impl Default for OptionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
