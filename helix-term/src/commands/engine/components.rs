use std::collections::HashMap;

use helix_core::Position;
use helix_view::{
    graphics::{Color, CursorKind, Rect, UnderlineStyle},
    input::{Event, KeyEvent},
    keyboard::{KeyCode, KeyModifiers},
    theme::Style,
    Editor,
};
use steel::{
    rvals::{Custom, FromSteelVal, IntoSteelVal},
    steel_vm::{
        builtin::BuiltInModule,
        engine::Engine,
        register_fn::{MarkerWrapper1, RegisterFn},
    },
    SteelVal,
};
use tui::{
    buffer::Buffer,
    widgets::{Block, BorderType, Borders, Widget},
};

use crate::{
    commands::{
        engine::steel::{BoxDynComponent, ENGINE},
        Context,
    },
    compositor::{self, Component},
    ctrl, key,
    ui::{overlay::overlaid, Popup, Prompt, PromptEvent},
};

use super::steel::WrappedDynComponent;

// TODO: Move the main configuration function to use this instead
pub fn helix_component_module() -> BuiltInModule {
    let mut module = BuiltInModule::new("helix/components".to_string());

    module
        .register_fn("frame-set-string!", buffer_set_string)
        .register_fn("position", Position::new)
        .register_fn("position-row", |position: &Position| position.row)
        .register_fn("position-col", |position: &Position| position.col)
        .register_fn("area", helix_view::graphics::Rect::new)
        .register_fn("area-x", |area: &helix_view::graphics::Rect| area.x)
        .register_fn("area-y", |area: &helix_view::graphics::Rect| area.y)
        .register_fn("area-width", |area: &helix_view::graphics::Rect| area.width)
        .register_fn("area-height", |area: &helix_view::graphics::Rect| {
            area.height
        })
        .register_fn("overlaid", |component: &mut WrappedDynComponent| {
            let inner: Option<Box<dyn Component>> = component
                .inner
                .take()
                .map(|x| Box::new(overlaid(BoxDynComponent::new(x))) as Box<dyn Component>);

            component.inner = inner;
        })
        .register_fn("block", || {
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
                .border_type(BorderType::Rounded)
                .style(Style::default().bg(Color::Black))
        })
        .register_fn(
            "block/render",
            |buf: &mut Buffer, area: Rect, block: Block| block.render(area, buf),
        )
        .register_fn("buffer/clear", Buffer::clear)
        .register_fn("buffer/clear-with", Buffer::clear_with)
        .register_value("Color/Reset", Color::Reset.into_steelval().unwrap())
        .register_value("Color/Black", Color::Black.into_steelval().unwrap())
        .register_value("Color/Red", Color::Red.into_steelval().unwrap())
        .register_value("Color/White", Color::White.into_steelval().unwrap())
        .register_value("Color/Green", Color::Green.into_steelval().unwrap())
        .register_value("Color/Yellow", Color::Yellow.into_steelval().unwrap())
        .register_value("Color/Blue", Color::Blue.into_steelval().unwrap())
        .register_value("Color/Magenta", Color::Magenta.into_steelval().unwrap())
        .register_value("Color/Cyan", Color::Cyan.into_steelval().unwrap())
        .register_value("Color/Gray", Color::Gray.into_steelval().unwrap())
        .register_value("Color/LightRed", Color::LightRed.into_steelval().unwrap())
        .register_value(
            "Color/LightGreen",
            Color::LightGreen.into_steelval().unwrap(),
        )
        .register_value(
            "Color/LightYellow",
            Color::LightYellow.into_steelval().unwrap(),
        )
        .register_value("Color/LightBlue", Color::LightBlue.into_steelval().unwrap())
        .register_value(
            "Color/LightMagenta",
            Color::LightMagenta.into_steelval().unwrap(),
        )
        .register_value("Color/LightCyan", Color::LightCyan.into_steelval().unwrap())
        .register_value("Color/LightGray", Color::LightGray.into_steelval().unwrap())
        .register_fn("Color/rgb", Color::Rgb)
        .register_fn("Color/Indexed", Color::Indexed)
        .register_fn("style-fg", Style::fg)
        .register_fn("style-bg", Style::bg)
        .register_fn("style-underline-color", Style::underline_color)
        .register_fn("style-underline-style", Style::underline_style)
        .register_value(
            "Underline/Reset",
            UnderlineStyle::Reset.into_steelval().unwrap(),
        )
        .register_value(
            "Underline/Line",
            UnderlineStyle::Line.into_steelval().unwrap(),
        )
        .register_value(
            "Underline/Curl",
            UnderlineStyle::Curl.into_steelval().unwrap(),
        )
        .register_value(
            "Underline/Dotted",
            UnderlineStyle::Dotted.into_steelval().unwrap(),
        )
        .register_value(
            "Underline/Dashed",
            UnderlineStyle::Dashed.into_steelval().unwrap(),
        )
        .register_value(
            "Underline/DoubleLine",
            UnderlineStyle::DoubleLine.into_steelval().unwrap(),
        )
        .register_fn("style", || Style::default())
        .register_value(
            "event-result/consume",
            SteelEventResult::Consumed.into_steelval().unwrap(),
        )
        .register_value(
            "event-result/ignore",
            SteelEventResult::Ignored.into_steelval().unwrap(),
        )
        .register_value(
            "event-result/close",
            SteelEventResult::Close.into_steelval().unwrap(),
        )
        // TODO: Use a reference here instead of passing by value.
        .register_fn("key-event-char", |event: Event| {
            if let Event::Key(event) = event {
                event.char()
            } else {
                None
            }
        })
        .register_fn("key-event-modifier", |event: Event| {
            if let Event::Key(KeyEvent { modifiers, .. }) = event {
                Some(modifiers.bits())
            } else {
                None
            }
        })
        .register_value(
            "key-modifier-ctrl",
            SteelVal::IntV(KeyModifiers::CONTROL.bits() as isize),
        )
        .register_value(
            "key-modifier-shift",
            SteelVal::IntV(KeyModifiers::SHIFT.bits() as isize),
        )
        .register_value(
            "key-modifier-alt",
            SteelVal::IntV(KeyModifiers::ALT.bits() as isize),
        )
        .register_fn("key-event-escape?", |event: Event| {
            if let Event::Key(KeyEvent {
                code: KeyCode::Esc, ..
            }) = event
            {
                true
            } else {
                false
            }
        })
        .register_fn("key-event-backspace?", |event: Event| {
            if let Event::Key(KeyEvent {
                code: KeyCode::Backspace,
                ..
            }) = event
            {
                true
            } else {
                false
            }
        })
        .register_fn("key-event-enter?", |event: Event| {
            if let Event::Key(KeyEvent {
                code: KeyCode::Enter,
                ..
            }) = event
            {
                true
            } else {
                false
            }
        });

    module
}

fn buffer_set_string(
    buffer: &mut tui::buffer::Buffer,
    x: u16,
    y: u16,
    string: steel::rvals::SteelString,
    style: Style,
) {
    buffer.set_string(x, y, string.as_str(), style)
}

/// A dynamic component, used for rendering
#[derive(Clone)]
pub struct SteelDynamicComponent {
    // TODO: currently the component id requires using a &'static str,
    // however in a world with dynamic components that might not be
    // the case anymore
    _name: String,
    // This _should_ be a struct, but in theory can be whatever you want. It will be the first argument
    // passed to the functions in the remainder of the struct.
    state: SteelVal,
    handle_event: Option<SteelVal>,
    should_update: Option<SteelVal>,
    render: SteelVal,
    cursor: Option<SteelVal>,
    required_size: Option<SteelVal>,

    // Cached key event; we keep this around so that when sending
    // events to the event handler, we can reuse the heap allocation
    // instead of re-allocating for every event (which might be a lot)
    key_event: Option<SteelVal>,
}

impl SteelDynamicComponent {
    pub fn new(
        name: String,
        state: SteelVal,
        render: SteelVal,
        h: HashMap<String, SteelVal>,
    ) -> Self {
        Self {
            _name: name,
            state,
            render,
            handle_event: h.get("handle_event").cloned(),
            should_update: h.get("should_update").cloned(),
            cursor: h.get("cursor").cloned(),
            required_size: h.get("required_size").cloned(),
            key_event: None,
        }
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

#[derive(Clone)]
enum SteelEventResult {
    Consumed,
    Ignored,
    Close,
}

impl Custom for SteelEventResult {}

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
        let thunk = |engine: &mut Engine, f| {
            engine.call_function_with_args(
                self.render.clone(),
                vec![self.state.clone(), area.into_steelval().unwrap(), f],
            )
        };

        ENGINE
            .with(|x| {
                x.borrow_mut()
                    .with_mut_reference::<tui::buffer::Buffer, tui::buffer::Buffer>(frame)
                    .with_mut_reference::<Context, Context>(&mut ctx)
                    .consume(|engine, args| {
                        let mut arg_iter = args.into_iter();

                        let buffer = arg_iter.next().unwrap();
                        let context = arg_iter.next().unwrap();

                        engine.update_value("*helix.cx*", context);

                        (thunk)(engine, buffer)
                    })
            })
            .unwrap();
    }

    // TODO: Pass in event as well? Need to have immutable reference type
    // Otherwise, we're gonna be in a bad spot. For now - just clone the object and pass it through.
    // Clong is _not_ ideal, but it might be all we can do for now.
    fn handle_event(
        &mut self,
        event: &Event,
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

            log::info!("Handling custom event: {:?}", event);

            match self.key_event.as_mut() {
                Some(SteelVal::Custom(key_event)) => {
                    // Save the headache, reuse the allocation
                    if let Some(inner) = steel::rvals::as_underlying_type_mut::<Event>(
                        key_event.borrow_mut().as_mut(),
                    ) {
                        *inner = event.clone();
                    }
                }

                None => {
                    self.key_event = Some(event.clone().into_steelval().unwrap());
                }
                _ => {
                    panic!("This event needs to stay as a steelval");
                }
            }

            // Pass the `state` object through - this can be used for storing the state of whatever plugin thing we're
            // attempting to render
            let thunk = |engine: &mut Engine| {
                engine.call_function_with_args(
                    handle_event.clone(),
                    vec![self.state.clone(), self.key_event.clone().unwrap()],
                )
            };

            let close_fn = compositor::EventResult::Consumed(Some(Box::new(
                |compositor: &mut compositor::Compositor, _| {
                    // remove the layer
                    compositor.pop();
                },
            )));

            let event = match event {
                Event::Key(event) => *event,
                _ => return compositor::EventResult::Ignored(None),
            };

            match ENGINE.with(|x| {
                x.borrow_mut()
                    .with_mut_reference::<Context, Context>(&mut ctx)
                    .consume(move |engine, arguments| {
                        let context = arguments[0].clone();
                        engine.update_value("*helix.cx*", context);

                        thunk(engine)
                    })
            }) {
                Ok(v) => {
                    let value = SteelEventResult::from_steelval(&v);

                    match value {
                        Ok(SteelEventResult::Close) => close_fn,
                        Ok(SteelEventResult::Consumed) => compositor::EventResult::Consumed(None),
                        Ok(SteelEventResult::Ignored) => compositor::EventResult::Ignored(None),
                        _ => match event {
                            ctrl!('c') | key!(Esc) => close_fn,
                            _ => compositor::EventResult::Ignored(None),
                        },
                    }
                }
                Err(_) => compositor::EventResult::Ignored(None),
            }
        } else {
            compositor::EventResult::Ignored(None)
        }
    }

    fn should_update(&self) -> bool {
        if let Some(should_update) = &self.should_update {
            match ENGINE.with(|x| {
                let res = x
                    .borrow_mut()
                    .call_function_with_args(should_update.clone(), vec![self.state.clone()]);

                res
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
        _ctx: &Editor,
    ) -> (
        Option<helix_core::Position>,
        helix_view::graphics::CursorKind,
    ) {
        log::info!("Calling cursor with area: {:?}", area);
        if let Some(cursor) = &self.cursor {
            // Pass the `state` object through - this can be used for storing the state of whatever plugin thing we're
            // attempting to render
            let thunk = |engine: &mut Engine| {
                engine.call_function_with_args(
                    cursor.clone(),
                    vec![self.state.clone(), area.into_steelval().unwrap()],
                )
            };

            let result = Option::<helix_core::Position>::from_steelval(
                &ENGINE.with(|x| thunk(&mut (x.borrow_mut())).unwrap()),
            );

            log::info!("Setting cursor at position: {:?}", result);

            (result.unwrap(), CursorKind::Block)
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
