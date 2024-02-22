//! `helix-event` contains systems that allow (often async) communication between
//! different editor components without strongly coupling them. Specifically
//! it allows defining synchronous hooks that run when certain editor events
//! occur.
//!
//! The core of the event system are hook callbacks and the [`Event`] trait. A
//! hook is essentially just a closure `Fn(event: &mut impl Event) -> Result<()>`
//! that gets called every time an appropriate event is dispatched. The implementation
//! details of the [`Event`] trait are considered private. The [`events`] macro is
//! provided which automatically declares event types. Similarly the `register_hook`
//! macro should be used to (safely) declare event hooks.
//!
//! Hooks run synchronously which can be advantageous since they can modify the
//! current editor state right away (for example to immediately hide the completion
//! popup). However, they can not contain their own state without locking since
//! they only receive immutable references. For handler that want to track state, do
//! expensive background computations or debouncing an [`AsyncHook`] is preferable.
//! Async hooks are based around a channels that receive events specific to
//! that `AsyncHook` (usually an enum). These events can be sent by synchronous
//! hooks. Due to some limitations around tokio channels the [`send_blocking`]
//! function exported in this crate should be used instead of the builtin
//! `blocking_send`.
//!
//! In addition to the core event system, this crate contains some message queues
//! that allow transfer of data back to the main event loop from async hooks and
//! hooks that may not have access to all application data (for example in helix-view).
//! This include the ability to control rendering ([`lock_frame`], [`request_redraw`]) and
//! display status messages ([`status`]).
//!
//! Hooks declared in helix-term can furthermore dispatch synchronous jobs to be run on the
//! main loop (including access to the compositor). Ideally that queue will be moved
//! to helix-view in the future if we manage to detach the compositor from its rendering backend.

use anyhow::Result;
pub use cancel::{cancelable_future, cancelation, CancelRx, CancelTx};
pub use debounce::{send_blocking, AsyncHook};
pub use redraw::{lock_frame, redraw_requested, request_redraw, start_frame, RenderLockGuard};
pub use registry::Event;

mod cancel;
mod debounce;
mod hook;
mod redraw;
mod registry;
#[doc(hidden)]
pub mod runtime;
pub mod status;

#[cfg(test)]
mod test;

pub fn register_event<E: Event + 'static>() {
    registry::with_mut(|registry| registry.register_event::<E>())
}

/// Registers a hook that will be called when an event of type `E` is dispatched.
/// This function should usually not be used directly, use the [`register_hook`]
/// macro instead.
///
///
/// # Safety
///
/// `hook` must be totally generic over all lifetime parameters of `E`. For
/// example if `E` was a known type `Foo<'a, 'b>`, then the correct trait bound
/// would be `F: for<'a, 'b, 'c> Fn(&'a mut Foo<'b, 'c>)`, but there is no way to
/// express that kind of constraint for a generic type with the Rust type system
/// as of this writing.
pub unsafe fn register_hook_raw<E: Event>(
    hook: impl Fn(&mut E) -> Result<()> + 'static + Send + Sync,
) {
    registry::with_mut(|registry| registry.register_hook(hook))
}

/// Register a hook solely by event name
pub fn register_dynamic_hook(
    hook: impl Fn() -> Result<()> + 'static + Send + Sync,
    id: &str,
) -> Result<()> {
    registry::with_mut(|reg| reg.register_dynamic_hook(hook, id))
}

pub fn dispatch(e: impl Event) {
    registry::with(|registry| registry.dispatch(e));
}

/// Macro to declare events
///
/// # Examples
///
/// ``` no-compile
/// events! {
///     FileWrite(&Path)
///     ViewScrolled{ view: View, new_pos: ViewOffset }
///     DocumentChanged<'a> { old_doc: &'a Rope, doc: &'a mut Document, changes: &'a ChangeSet  }
/// }
///
/// fn init() {
///    register_event::<FileWrite>();
///    register_event::<ViewScrolled>();
///    register_event::<DocumentChanged>();
/// }
///
/// fn save(path: &Path, content: &str){
///     std::fs::write(path, content);
///     dispatch(FileWrite(path));
/// }
/// ```
#[macro_export]
macro_rules! events {
    ($name: ident<$($lt: lifetime),*> { $($data:ident : $data_ty:ty),* } $($rem:tt)*) => {
        pub struct $name<$($lt),*> { $(pub $data: $data_ty),* }
        unsafe impl<$($lt),*> $crate::Event for $name<$($lt),*> {
            const ID: &'static str = stringify!($name);
            const LIFETIMES: usize = $crate::events!(@sum $(1, $lt),*);
            type Static = $crate::events!(@replace_lt $name, $('static, $lt),*);
        }
        $crate::events!{ $($rem)* }
    };
    ($name: ident { $($data:ident : $data_ty:ty),* } $($rem:tt)*) => {
        pub struct $name { $(pub $data: $data_ty),* }
        unsafe impl $crate::Event for $name {
            const ID: &'static str = stringify!($name);
            const LIFETIMES: usize = 0;
            type Static = Self;
        }
        $crate::events!{ $($rem)* }
    };
    () => {};
    (@replace_lt $name: ident, $($lt1: lifetime, $lt2: lifetime),* ) => {$name<$($lt1),*>};
    (@sum $($val: expr, $lt1: lifetime),* ) => {0 $(+ $val)*};
}

/// Safely register statically typed event hooks
#[macro_export]
macro_rules! register_hook {
    // Safety: this is safe because we fully control the type of the event here and
    // ensure all lifetime arguments are fully generic and the correct number of lifetime arguments
    // is present
    (move |$event:ident: &mut $event_ty: ident<$($lt: lifetime),*>| $body: expr) => {
        let val = move |$event: &mut $event_ty<$($lt),*>| $body;
        unsafe {
            // Lifetimes are a bit of a pain. We want to allow events being
            // non-static. Lifetimes don't actually exist at runtime so its
            // fine to essentially transmute the lifetimes as long as we can
            // prove soundness. The hook must therefore accept any combination
            // of lifetimes. In other words fn(&'_ mut Event<'_, '_>) is ok
            // but examples like fn(&'_ mut Event<'_, 'static>) or fn<'a>(&'a
            // mut Event<'a, 'a>) are not. To make this safe we use a macro to
            // forbid the user from specifying lifetimes manually (all lifetimes
            // specified are always function generics and passed to the event so
            // lifetimes can't be used multiple times and using 'static causes a
            // syntax error).
            //
            // There is one soundness hole tough: Type Aliases allow
            // "accidentally" creating these problems. For example:
            //
            // type Event2  = Event<'static>.
            // type Event2<'a>  = Event<'a, a>.
            //
            // These cases can be caught by counting the number of lifetimes
            // parameters at the parameter declaration site and then at the hook
            // declaration site. By asserting the number of lifetime parameters
            // are equal we can catch all bad type aliases under one assumption:
            // There are no unused lifetime parameters. Introducing a static
            // would reduce the number of arguments of the alias by one in the
            // above example Event2 has zero lifetime arguments while the original
            // event has one lifetime argument. Similar logic applies to using
            // a lifetime argument multiple times. The ASSERT below performs a
            // a compile time assertion to ensure exactly this property.
            //
            // With unused lifetime arguments it is still one way to cause unsound code:
            //
            // type Event2<'a, 'b> = Event<'a, 'a>;
            //
            // However, this case will always emit a compiler warning/cause CI
            // failures so a user would have to introduce #[allow(unused)] which
            // is easily caught in review (and a very theoretical case anyway).
            // If we want to be pedantic we can simply compile helix with
            // forbid(unused). All of this is just a safety net to prevent
            // very theoretical misuse. This won't come up in real code (and is
            // easily caught in review).
            #[allow(unused)]
            const ASSERT: () = {
                if <$event_ty as $crate::Event>::LIFETIMES != 0 + $crate::events!(@sum $(1, $lt),*){
                    panic!("invalid type alias");
                }
            };
            $crate::register_hook_raw::<$crate::events!(@replace_lt $event_ty, $('static, $lt),*)>(val);
        }
    };
    (move |$event:ident: &mut $event_ty: ident| $body: expr) => {
        let val = move |$event: &mut $event_ty| $body;
        unsafe {
            #[allow(unused)]
            const ASSERT: () = {
                if <$event_ty as $crate::Event>::LIFETIMES != 0{
                    panic!("invalid type alias");
                }
            };
            $crate::register_hook_raw::<$event_ty>(val);
        }
    };
}
