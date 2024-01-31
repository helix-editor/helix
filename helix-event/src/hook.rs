//! rust dynamic dispatch is extremely limited so we have to build our
//! own vtable implementation. Otherwise implementing the event system would not be possible.
//! A nice bonus of this approach is that we can optimize the vtable a bit more. Normally
//! a dyn Trait fat pointer contains two pointers: A pointer to the data itself and a
//! pointer to a global (static) vtable entry which itself contains multiple other pointers
//! (the various functions of the trait, drop, size and align). That makes dynamic
//! dispatch pretty slow (double pointer indirections). However, we only have a single function
//! in the hook trait and don't need a drop implementation (event system is global anyway
//! and never dropped) so we can just store the entire vtable inline.

use anyhow::Result;
use std::ptr::{self, NonNull};

use crate::Event;

/// Opaque handle type that represents an erased type parameter.
///
/// If extern types were stable, this could be implemented as `extern { pub type Opaque; }` but
/// until then we can use this.
///
/// Care should be taken that we don't use a concrete instance of this. It should only be used
/// through a reference, so we can maintain something else's lifetime.
struct Opaque(());

pub(crate) struct ErasedHook {
    data: NonNull<Opaque>,
    call: unsafe fn(NonNull<Opaque>, NonNull<Opaque>, NonNull<Opaque>),
}

impl ErasedHook {
    pub(crate) fn new_dynamic<H: Fn() -> Result<()> + 'static + Send + Sync>(
        hook: H,
    ) -> ErasedHook {
        unsafe fn call<F: Fn() -> Result<()> + 'static + Send + Sync>(
            hook: NonNull<Opaque>,
            _event: NonNull<Opaque>,
            result: NonNull<Opaque>,
        ) {
            let hook: NonNull<F> = hook.cast();
            let result: NonNull<Result<()>> = result.cast();
            let hook: &F = hook.as_ref();
            let res = hook();
            ptr::write(result.as_ptr(), res)
        }

        unsafe {
            ErasedHook {
                data: NonNull::new_unchecked(Box::into_raw(Box::new(hook)) as *mut Opaque),
                call: call::<H>,
            }
        }
    }

    pub(crate) fn new<E: Event, F: Fn(&mut E) -> Result<()>>(hook: F) -> ErasedHook {
        unsafe fn call<E: Event, F: Fn(&mut E) -> Result<()>>(
            hook: NonNull<Opaque>,
            event: NonNull<Opaque>,
            result: NonNull<Opaque>,
        ) {
            let hook: NonNull<F> = hook.cast();
            let mut event: NonNull<E> = event.cast();
            let result: NonNull<Result<()>> = result.cast();
            let hook: &F = hook.as_ref();
            let res = hook(event.as_mut());
            ptr::write(result.as_ptr(), res)
        }

        unsafe {
            ErasedHook {
                data: NonNull::new_unchecked(Box::into_raw(Box::new(hook)) as *mut Opaque),
                call: call::<E, F>,
            }
        }
    }

    pub(crate) unsafe fn call<E: Event>(&self, event: &mut E) -> Result<()> {
        let mut res = Ok(());

        unsafe {
            (self.call)(
                self.data,
                NonNull::from(event).cast(),
                NonNull::from(&mut res).cast(),
            );
        }
        res
    }
}

unsafe impl Sync for ErasedHook {}
unsafe impl Send for ErasedHook {}
