use std::collections::HashMap;

use helix_view::Editor;
use steel::{
    rvals::{Custom, FromSteelVal, IntoSteelVal},
    steel_vm::{builtin::BuiltInModule, engine::Engine, register_fn::RegisterFn},
    SteelVal,
};

use crate::{
    commands::{engine::scheme::ENGINE, Context},
    compositor::{self, Component},
    ui::{Popup, Prompt, PromptEvent},
};

// TODO: Move the main configuration function to use this instead
pub fn helix_component_module() -> BuiltInModule {
    let mut module = BuiltInModule::new("helix/components".to_string());

    module
}

/// A dynamic component, used for rendering thing
#[derive(Clone)]
// TODO: Implement `trace` method for objects that hold steel vals
pub struct SteelDynamicComponent {
    name: String,
    // This _should_ be a struct, but in theory can be whatever you want. It will be the first argument
    // passed to the functions in the remainder of the struct.
    state: SteelVal,
    handle_event: Option<SteelVal>,
    should_update: Option<SteelVal>,
    render: SteelVal,
    cursor: Option<SteelVal>,
    required_size: Option<SteelVal>,
}

impl SteelDynamicComponent {
    pub fn new(
        name: String,
        state: SteelVal,
        render: SteelVal,
        h: HashMap<String, SteelVal>,
    ) -> Self {
        // if let SteelVal::HashMapV(h) = functions {

        Self {
            name,
            state,
            render,
            handle_event: h.get("handle_event").cloned(),
            should_update: h.get("should_update").cloned(),
            cursor: h.get("cursor").cloned(),
            required_size: h.get("required_size").cloned(),
        }

        // } else {
        // panic!("Implement better error handling")
        // }
    }

    pub fn new_dyn(
        name: String,
        state: SteelVal,
        render: SteelVal,
        h: HashMap<String, SteelVal>,
    ) -> WrappedDynComponent {
        let s = Self::new(name, state, render, h);

        // TODO: Add guards here for the
        WrappedDynComponent {
            inner: Some(Box::new(s)),
        }
    }

    pub fn get_state(&self) -> SteelVal {
        self.state.clone()
    }

    pub fn get_render(&self) -> SteelVal {
        self.render.clone()
    }

    pub fn get_handle_event(&self) -> Option<SteelVal> {
        self.handle_event.clone()
    }

    pub fn get_should_update(&self) -> Option<SteelVal> {
        self.should_update.clone()
    }

    pub fn get_cursor(&self) -> Option<SteelVal> {
        self.cursor.clone()
    }

    pub fn get_required_size(&self) -> Option<SteelVal> {
        self.required_size.clone()
    }
}

impl Custom for SteelDynamicComponent {}

impl Custom for Box<dyn Component> {}

pub struct WrappedDynComponent {
    inner: Option<Box<dyn Component>>,
}

impl Custom for WrappedDynComponent {}

struct BoxDynComponent {
    inner: Box<dyn Component>,
}

impl BoxDynComponent {
    pub fn new(inner: Box<dyn Component>) -> Self {
        Self { inner }
    }
}

impl Component for BoxDynComponent {
    fn handle_event(
        &mut self,
        _event: &helix_view::input::Event,
        _ctx: &mut compositor::Context,
    ) -> compositor::EventResult {
        self.inner.handle_event(_event, _ctx)
    }

    fn should_update(&self) -> bool {
        self.inner.should_update()
    }

    fn cursor(
        &self,
        _area: helix_view::graphics::Rect,
        _ctx: &Editor,
    ) -> (
        Option<helix_core::Position>,
        helix_view::graphics::CursorKind,
    ) {
        self.inner.cursor(_area, _ctx)
    }

    fn required_size(&mut self, _viewport: (u16, u16)) -> Option<(u16, u16)> {
        self.inner.required_size(_viewport)
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn id(&self) -> Option<&'static str> {
        None
    }

    fn render(
        &mut self,
        area: helix_view::graphics::Rect,
        frame: &mut tui::buffer::Buffer,
        ctx: &mut compositor::Context,
    ) {
        self.inner.render(area, frame, ctx)
    }
}
impl Component for SteelDynamicComponent {
    fn render(
        &mut self,
        area: helix_view::graphics::Rect,
        frame: &mut tui::buffer::Buffer,
        ctx: &mut compositor::Context,
    ) {
        let mut ctx = Context {
            register: None,
            count: None,
            editor: ctx.editor,
            callback: Vec::new(),
            on_next_key_callback: None,
            jobs: ctx.jobs,
        };

        // Pass the `state` object through - this can be used for storing the state of whatever plugin thing we're
        // attempting to render
        let thunk = |engine: &mut Engine, f, c| {
            engine.call_function_with_args(
                self.render.clone(),
                vec![self.state.clone(), area.into_steelval().unwrap(), f, c],
            )
        };

        ENGINE
            .with(|x| {
                x.borrow_mut()
                    .with_mut_reference::<tui::buffer::Buffer, tui::buffer::Buffer>(frame)
                    .with_mut_reference::<Context, Context>(&mut ctx)
                    .consume(|engine, args| {
                        let mut arg_iter = args.into_iter();

                        (thunk)(engine, arg_iter.next().unwrap(), arg_iter.next().unwrap())
                    })

                // .run_with_references::<tui::buffer::Buffer, tui::buffer::Buffer, Context, Context>(
                //     frame, &mut ctx, thunk,
                // )
            })
            .unwrap();

        log::info!("Calling dynamic render!");
    }

    // TODO: Pass in event as well? Need to have immutable reference type
    // Otherwise, we're gonna be in a bad spot. For now - just clone the object and pass it through.
    // Clong is _not_ ideal, but it might be all we can do for now.
    fn handle_event(
        &mut self,
        event: &helix_view::input::Event,
        ctx: &mut compositor::Context,
    ) -> compositor::EventResult {
        if let Some(handle_event) = &mut self.handle_event {
            let mut ctx = Context {
                register: None,
                count: None,
                editor: ctx.editor,
                callback: Vec::new(),
                on_next_key_callback: None,
                jobs: ctx.jobs,
            };

            // Pass the `state` object through - this can be used for storing the state of whatever plugin thing we're
            // attempting to render
            let thunk = |engine: &mut Engine, c| {
                engine.call_function_with_args(
                    handle_event.clone(),
                    vec![
                        self.state.clone(),
                        // TODO: We do _not_ want to clone here, we would need to create a bunch of methods on the engine for various
                        // combinations of reference passing to do this safely. Right now its limited to mutable references, but we should
                        // expose more - investigate macros on how to do that with recursively crunching the list to generate the combinations.
                        // Experimentation needed.
                        event.clone().into_steelval().unwrap(),
                        c,
                    ],
                )
            };

            match ENGINE.with(|x| {
                x.borrow_mut()
                    .run_thunk_with_reference::<Context, Context>(&mut ctx, thunk)
            }) {
                Ok(v) => compositor::EventResult::from_steelval(&v)
                    .unwrap_or_else(|_| compositor::EventResult::Ignored(None)),
                Err(_) => compositor::EventResult::Ignored(None),
            }
        } else {
            compositor::EventResult::Ignored(None)
        }
    }

    fn should_update(&self) -> bool {
        if let Some(should_update) = &self.should_update {
            match ENGINE.with(|x| {
                x.borrow_mut()
                    .call_function_with_args(should_update.clone(), vec![self.state.clone()])
            }) {
                Ok(v) => bool::from_steelval(&v).unwrap_or(true),
                Err(_) => true,
            }
        } else {
            true
        }
    }

    // TODO: Implement immutable references. Right now I'm only supporting mutable references.
    fn cursor(
        &self,
        area: helix_view::graphics::Rect,
        ctx: &Editor,
    ) -> (
        Option<helix_core::Position>,
        helix_view::graphics::CursorKind,
    ) {
        if let Some(cursor) = &self.cursor {
            // Pass the `state` object through - this can be used for storing the state of whatever plugin thing we're
            // attempting to render
            let thunk = |engine: &mut Engine, e| {
                engine.call_function_with_args(
                    cursor.clone(),
                    vec![self.state.clone(), area.into_steelval().unwrap(), e],
                )
            };

            <(
                Option<helix_core::Position>,
                helix_view::graphics::CursorKind,
            )>::from_steelval(&ENGINE.with(|x| {
                x.borrow_mut()
                    .run_thunk_with_ro_reference::<Editor, Editor>(ctx, thunk)
                    .unwrap()
            }))
            .unwrap()
        } else {
            (None, helix_view::graphics::CursorKind::Hidden)
        }
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        let name = self.type_name();

        if let Some(required_size) = &mut self.required_size {
            log::info!("Calling required-size inside: {}", name);

            // TODO: Create some token that we can grab to enqueue function calls internally. Referencing
            // the external API would cause problems - we just need to include a handle to the interpreter
            // instance. Something like:
            // ENGINE.call_function_or_enqueue? OR - this is the externally facing render function. Internal
            // render calls do _not_ go through this interface. Instead, they are just called directly.
            //
            // If we go through this interface, we're going to get an already borrowed mut error, since it is
            // re-entrant attempting to grab the ENGINE instead mutably, since we have to break the recursion
            // somehow. By putting it at the edge, we then say - hey for these functions on this interface,
            // call the engine instance. Otherwise, all computation happens inside the engine.
            let res = ENGINE
                .with(|x| {
                    x.borrow_mut().call_function_with_args(
                        required_size.clone(),
                        vec![self.state.clone(), viewport.into_steelval().unwrap()],
                    )
                })
                .and_then(|x| Option::<(u16, u16)>::from_steelval(&x))
                .unwrap();

            res
        } else {
            None
        }
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn id(&self) -> Option<&'static str> {
        None
    }
}
