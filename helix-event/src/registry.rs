//! A global registry where events are registered and can be
//! subscribed to by registering hooks. The registry identifies event
//! types using their type name so multiple event with the same type name
//! may not be registered (will cause a panic to ensure soundness)

use std::any::TypeId;

use anyhow::{bail, Result};
use hashbrown::hash_map::Entry;
use hashbrown::HashMap;
use parking_lot::RwLock;

use crate::hook::ErasedHook;
use crate::runtime_local;

pub struct Registry {
    events: HashMap<&'static str, TypeId, ahash::RandomState>,
    handlers: HashMap<&'static str, Vec<ErasedHook>, ahash::RandomState>,
}

impl Registry {
    pub fn register_event<E: Event + 'static>(&mut self) {
        let ty = TypeId::of::<E>();
        assert_eq!(ty, TypeId::of::<E::Static>());
        match self.events.entry(E::ID) {
            Entry::Occupied(entry) => {
                if entry.get() == &ty {
                    // don't warn during tests to avoid log spam
                    #[cfg(not(feature = "integration_test"))]
                    panic!("Event {} was registered multiple times", E::ID);
                } else {
                    panic!("Multiple events with ID {} were registered", E::ID);
                }
            }
            Entry::Vacant(ent) => {
                ent.insert(ty);
                self.handlers.insert(E::ID, Vec::new());
            }
        }
    }

    /// # Safety
    ///
    /// `hook` must be totally generic over all lifetime parameters of `E`. For
    /// example if `E` was a known type `Foo<'a, 'b> then the correct trait bound
    /// would be `F: for<'a, 'b, 'c> Fn(&'a mut Foo<'b, 'c>)` but there is no way to
    /// express that kind of constraint for a generic type with the rust type system
    /// right now.
    pub unsafe fn register_hook<E: Event>(
        &mut self,
        hook: impl Fn(&mut E) -> Result<()> + 'static + Send + Sync,
    ) {
        // ensure event type ids match so we can rely on them always matching
        let id = E::ID;
        let Some(&event_id) = self.events.get(id) else {
            panic!("Tried to register handler for unknown event {id}");
        };
        assert!(
            TypeId::of::<E::Static>() == event_id,
            "Tried to register invalid hook for event {id}"
        );
        let hook = ErasedHook::new(hook);
        self.handlers.get_mut(id).unwrap().push(hook);
    }

    pub fn register_dynamic_hook(
        &mut self,
        hook: impl Fn() -> Result<()> + 'static + Send + Sync,
        id: &str,
    ) -> Result<()> {
        // ensure event type ids match so we can rely on them always matching
        if self.events.get(id).is_none() {
            bail!("Tried to register handler for unknown event {id}");
        };
        let hook = ErasedHook::new_dynamic(hook);
        self.handlers.get_mut(id).unwrap().push(hook);
        Ok(())
    }

    pub fn dispatch<E: Event>(&self, mut event: E) {
        let Some(hooks) = self.handlers.get(E::ID) else {
            log::error!("Dispatched unknown event {}", E::ID);
            return;
        };
        let event_id = self.events[E::ID];

        assert_eq!(
            TypeId::of::<E::Static>(),
            event_id,
            "Tried to dispatch invalid event {}",
            E::ID
        );

        for hook in hooks {
            // safety: event type is the same
            if let Err(err) = unsafe { hook.call(&mut event) } {
                log::error!("{} hook failed: {err:#?}", E::ID);
                crate::status::report_blocking(err);
            }
        }
    }
}

runtime_local! {
    static REGISTRY: RwLock<Registry> = RwLock::new(Registry {
        // hardcoded random number is good enough here we don't care about DOS resistance
        // and avoids the additional complexity of `Option<Registry>`
        events: HashMap::with_hasher(ahash::RandomState::with_seeds(423, 9978, 38322, 3280080)),
        handlers: HashMap::with_hasher(ahash::RandomState::with_seeds(423, 99078, 382322, 3282938)),
    });
}

pub(crate) fn with<T>(f: impl FnOnce(&Registry) -> T) -> T {
    f(&REGISTRY.read())
}

pub(crate) fn with_mut<T>(f: impl FnOnce(&mut Registry) -> T) -> T {
    f(&mut REGISTRY.write())
}

/// # Safety
/// The number of specified lifetimes and the static type *must* be correct.
/// This is ensured automatically by the [`events`](crate::events)
/// macro.
pub unsafe trait Event: Sized {
    /// Globally unique (case sensitive)  string that identifies this type.
    /// A good candidate is the events type name
    const ID: &'static str;
    const LIFETIMES: usize;
    type Static: Event + 'static;
}
