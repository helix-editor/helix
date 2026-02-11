use std::{collections::HashMap, str::FromStr, sync::Arc};

use helix_core::Position;
use helix_view::{
    graphics::{Color, CursorKind, Rect, UnderlineStyle},
    input::{Event, KeyEvent, MouseButton, MouseEvent},
    keyboard::{KeyCode, KeyModifiers},
    theme::{Modifier, Style},
    Editor,
};
use steel::{
    rvals::{as_underlying_type, AsRefSteelVal, Custom, FromSteelVal, IntoSteelVal, SteelString},
    steel_vm::{builtin::BuiltInModule, engine::Engine, register_fn::RegisterFn},
    RootedSteelVal, SteelVal,
};
use tokio::sync::Mutex;
use tui::{
    buffer::Buffer,
    text::Text,
    widgets::{self, Block, BorderType, Borders, ListItem, Widget},
};

use crate::{
    commands::{
        engine::steel::{configure_lsp_builtins, generate_module, BoxDynComponent},
        Context,
    },
    compositor::{self, Component},
    ui::{self, overlay::overlaid},
};

use super::{
    enter_engine, format_docstring, is_current_generation, load_generation,
    present_error_inside_engine_context, with_context_guard, WrappedDynComponent, CTX,
};

#[derive(Clone)]
struct AsyncReader {
    // Take that, and write it back to a terminal session that is
    // getting rendered.
    channel: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<String>>>,
}

impl AsyncReader {
    async fn read_line(self) -> Option<String> {
        let mut buf = String::new();

        let mut guard = self.channel.lock().await;

        while let Ok(v) = guard.try_recv() {
            buf.push_str(&v);
        }

        let fut = guard.recv();

        // If we haven't found any characters, just wait until we have something.
        // Otherwise, we give this a 2 ms buffer to check if more things are
        // coming through the pipe.
        if buf.is_empty() {
            let next = fut.await;

            match next {
                Some(v) => {
                    buf.push_str(&v);
                    Some(buf)
                }
                None => None,
            }
        } else {
            match tokio::time::timeout(std::time::Duration::from_millis(2), fut).await {
                Ok(Some(v)) => {
                    buf.push_str(&v);
                    Some(buf)
                }
                Ok(None) => {
                    if buf.is_empty() {
                        None
                    } else {
                        Some(buf)
                    }
                }
                Err(_) => Some(buf),
            }
        }
    }
}

impl Custom for AsyncReader {}

struct AsyncWriter {
    channel: tokio::sync::mpsc::UnboundedSender<String>,
}

impl std::io::Write for AsyncWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self
            .channel
            .send(String::from_utf8_lossy(buf).into_owned())
            .is_err()
        {
            Ok(0)
        } else {
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn helix_component_module(generate_sources: bool) -> BuiltInModule {
    let mut module = BuiltInModule::new("helix/components");

    let mut builtin_components_module = include_str!("components.scm").to_string();

    module
        .register_fn("async-read-line", AsyncReader::read_line)
        .register_fn("make-async-reader-writer", || {
            let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

            let writer = AsyncWriter { channel: sender };
            let reader = AsyncReader {
                channel: Arc::new(Mutex::new(receiver)),
            };

            vec![
                SteelVal::new_dyn_writer_port(writer),
                reader.into_steelval().unwrap(),
            ]
        })
        .register_fn("theme->bg", |ctx: &mut Context| {
            ctx.editor.theme.get("ui.background")
        })
        .register_fn("theme->fg", |ctx: &mut Context| {
            ctx.editor.theme.get("ui.text")
        })
        .register_fn_with_ctx(
            CTX,
            "theme-scope-ref",
            |ctx: &mut Context, scope: SteelString| ctx.editor.theme.get(scope.as_str()),
        )
        .register_fn("theme-scope", |ctx: &mut Context, scope: SteelString| {
            ctx.editor.theme.get(scope.as_str())
        })
        .register_fn("Position?", |position: SteelVal| {
            Position::as_ref(&position).is_ok()
        })
        .register_fn("Style?", |style: SteelVal| Style::as_ref(&style).is_ok())
        .register_fn("Buffer?", |value: SteelVal| {
            steel::gc::is_reference_type::<Buffer>(&value)
        })
        .register_fn("buffer-area", |buffer: &mut Buffer| buffer.area)
        .register_fn("frame-set-string!", buffer_set_string)
        .register_fn("SteelEventResult?", |value: SteelVal| {
            SteelEventResult::as_ref(&value).is_ok()
        })
        .register_fn("new-component!", SteelDynamicComponent::new_dyn)
        .register_fn("position", Position::new)
        .register_fn("position-row", |position: &Position| position.row)
        .register_fn("position-col", |position: &Position| position.col)
        .register_fn(
            "set-position-row!",
            |position: &mut Position, row: usize| {
                position.row = row;
            },
        )
        .register_fn(
            "set-position-col!",
            |position: &mut Position, col: usize| {
                position.col = col;
            },
        )
        .register_fn("Rect?", |value: SteelVal| Rect::as_ref(&value).is_ok())
        .register_fn("area", helix_view::graphics::Rect::new)
        .register_fn("area-x", |area: &helix_view::graphics::Rect| area.x)
        .register_fn("area-y", |area: &helix_view::graphics::Rect| area.y)
        .register_fn("area-width", |area: &helix_view::graphics::Rect| area.width)
        .register_fn("area-height", |area: &helix_view::graphics::Rect| {
            area.height
        })
        .register_fn(
            "render-native-component",
            |this: &mut WrappedDynComponent,
             area: helix_view::graphics::Rect,
             frame: &mut Buffer,
             ctx: &mut Context| {
                if let Some(inner) = &mut this.inner {
                    let mut ctx = compositor::Context {
                        editor: ctx.editor,
                        scroll: None,
                        jobs: ctx.jobs,
                    };

                    inner.render(area, frame, &mut ctx);
                }
            },
        )
        .register_fn(
            "native-component-required-size",
            |this: &mut WrappedDynComponent, viewport: (u16, u16)| -> Option<(u16, u16)> {
                if let Some(inner) = &mut this.inner {
                    return inner.required_size(viewport);
                }

                None
            },
        )
        .register_fn_with_ctx(
            CTX,
            "markdown-component",
            |ctx: &mut Context, contents: String| {
                let markdown = ui::Markdown::new(contents, ctx.editor.syn_loader.clone());

                WrappedDynComponent {
                    inner: Some(Box::new(markdown)),
                }
            },
        )
        .register_fn("overlaid", |component: &mut WrappedDynComponent| {
            let inner: Option<Box<dyn Component + Send + Sync + 'static>> =
                component.inner.take().map(|x| {
                    Box::new(overlaid(BoxDynComponent::new(x)))
                        as Box<dyn Component + Send + Sync + 'static>
                });

            component.inner = inner;
        })
        .register_fn("Widget/list?", |value: SteelVal| {
            widgets::List::as_ref(&value).is_ok()
        })
        .register_fn("widget/list", |items: Vec<String>| {
            widgets::List::new(
                items
                    .into_iter()
                    .map(|x| ListItem::new(Text::from(x)))
                    .collect::<Vec<_>>(),
            )
        })
        .register_fn(
            "widget/list/render",
            |buf: &mut Buffer, area: Rect, list: widgets::List| list.render(area, buf),
        )
        .register_fn("block", || {
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
                .border_type(BorderType::Rounded)
                .style(Style::default().bg(Color::Black))
        })
        .register_fn(
            "make-block",
            |style: Style, border_style: Style, borders: SteelString, border_type: SteelString| {
                let border_type = match border_type.as_str() {
                    "plain" => BorderType::Plain,
                    "rounded" => BorderType::Rounded,
                    "double" => BorderType::Double,
                    "thick" => BorderType::Thick,
                    _ => BorderType::Plain,
                };

                let borders = match borders.as_str() {
                    "top" => Borders::TOP,
                    "left" => Borders::LEFT,
                    "right" => Borders::RIGHT,
                    "bottom" => Borders::BOTTOM,
                    "all" => Borders::ALL,
                    _ => Borders::empty(),
                };

                Block::default()
                    .borders(borders)
                    .border_style(border_style)
                    .border_type(border_type)
                    .style(style)
            },
        )
        .register_fn(
            "block/render",
            |buf: &mut Buffer, area: Rect, block: Block| {
                block.render_with_bounds(area, buf);
            },
        )
        .register_fn("buffer/clear", |buffer: &mut Buffer, area: Rect| {
            for x in area.left()..area.right() {
                for y in area.top()..area.bottom() {
                    if let Some(cell) = buffer.get_mut(x, y) {
                        cell.reset()
                    };
                }
            }
        })
        .register_fn(
            "buffer/clear-with",
            |buffer: &mut Buffer, area: Rect, style: Style| {
                for x in area.left()..area.right() {
                    for y in area.top()..area.bottom() {
                        let cell = buffer.get_mut(x, y);
                        if let Some(cell) = cell {
                            cell.reset();
                            cell.set_style(style);
                        }
                    }
                }
            },
        )
        // Mutate a color in place, to save some headache.
        .register_fn(
            "set-color-rgb!",
            |color: &mut Color, r: u8, g: u8, b: u8| {
                *color = Color::Rgb(r, g, b);
            },
        )
        .register_fn("set-color-indexed!", |color: &mut Color, index: u8| {
            *color = Color::Indexed(index);
        })
        .register_fn("Color?", |color: SteelVal| Color::as_ref(&color).is_ok())
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
        .register_fn("Color-red", Color::red)
        .register_fn("Color-green", Color::green)
        .register_fn("Color-blue", Color::blue)
        .register_fn("Color/Indexed", Color::Indexed)
        .register_fn("set-style-fg!", |style: &mut Style, color: Color| {
            style.fg = Some(color);
        })
        .register_fn("style-fg", Style::fg)
        .register_fn("style-bg", Style::bg)
        .register_fn("style-with-italics", |style: &Style| {
            let patch = Style::default().add_modifier(Modifier::ITALIC);
            style.patch(patch)
        })
        .register_fn("style-with-bold", |style: Style| {
            let patch = Style::default().add_modifier(Modifier::BOLD);
            style.patch(patch)
        })
        .register_fn("style-with-dim", |style: &Style| {
            let patch = Style::default().add_modifier(Modifier::DIM);
            style.patch(patch)
        })
        .register_fn("style-with-slow-blink", |style: Style| {
            let patch = Style::default().add_modifier(Modifier::SLOW_BLINK);
            style.patch(patch)
        })
        .register_fn("style-with-rapid-blink", |style: Style| {
            let patch = Style::default().add_modifier(Modifier::RAPID_BLINK);
            style.patch(patch)
        })
        .register_fn("style-with-reversed", |style: Style| {
            let patch = Style::default().add_modifier(Modifier::REVERSED);
            style.patch(patch)
        })
        .register_fn("style-with-hidden", |style: Style| {
            let patch = Style::default().add_modifier(Modifier::HIDDEN);
            style.patch(patch)
        })
        .register_fn("style-with-crossed-out", |style: Style| {
            let patch = Style::default().add_modifier(Modifier::CROSSED_OUT);
            style.patch(patch)
        })
        .register_fn("style->fg", |style: &Style| style.fg)
        .register_fn("style->bg", |style: &Style| style.bg)
        .register_fn("set-style-bg!", |style: &mut Style, color: Color| {
            style.bg = Some(color);
        })
        .register_fn("style-underline-color", Style::underline_color)
        .register_fn("style-underline-style", Style::underline_style)
        .register_fn("UnderlineStyle?", |value: SteelVal| {
            UnderlineStyle::as_ref(&value).is_ok()
        })
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
        .register_value(
            "event-result/consume",
            SteelEventResult::Consumed.into_steelval().unwrap(),
        )
        .register_value(
            "event-result/consume-without-rerender",
            SteelEventResult::ConsumedWithoutRerender
                .into_steelval()
                .unwrap(),
        )
        .register_value(
            "event-result/ignore",
            SteelEventResult::Ignored.into_steelval().unwrap(),
        )
        .register_value(
            "event-result/ignore-and-close",
            SteelEventResult::IgnoreAndClose.into_steelval().unwrap(),
        )
        .register_value(
            "event-result/close",
            SteelEventResult::Close.into_steelval().unwrap(),
        )
        .register_fn("style", Style::default)
        .register_fn("Event?", |value: SteelVal| Event::as_ref(&value).is_ok())
        .register_fn("paste-event?", |event: Event| {
            matches!(event, Event::Paste(_))
        })
        .register_fn("paste-event-string", |event: Event| {
            if let Event::Paste(p) = event {
                Some(p)
            } else {
                None
            }
        })
        .register_fn("key-event?", |event: Event| matches!(event, Event::Key(_)))
        .register_fn("string->key-event", |value: SteelString| {
            KeyEvent::from_str(value.as_str())
        })
        .register_fn("event->key-event", |event: Event| {
            if let Event::Key(k) = event {
                Some(k)
            } else {
                None
            }
        })
        .register_fn("key-event-char", |event: Event| {
            if let Event::Key(event) = event {
                event.char()
            } else {
                None
            }
        })
        .register_fn("on-key-event-char", |event: KeyEvent| event.char())
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
        .register_value(
            "key-modifier-super",
            SteelVal::IntV(KeyModifiers::SUPER.bits() as isize),
        )
        .register_fn("key-event-F?", |event: Event, number: u8| match event {
            Event::Key(KeyEvent {
                code: KeyCode::F(x),
                ..
            }) => number == x,
            _ => false,
        })
        .register_fn("mouse-event?", |event: Event| {
            matches!(event, Event::Mouse(_))
        })
        .register_fn("event-mouse-kind", |event: Event| {
            if let Event::Mouse(MouseEvent { kind, .. }) = event {
                match kind {
                    helix_view::input::MouseEventKind::Down(MouseButton::Left) => 0,
                    helix_view::input::MouseEventKind::Down(MouseButton::Right) => 1,
                    helix_view::input::MouseEventKind::Down(MouseButton::Middle) => 2,
                    helix_view::input::MouseEventKind::Up(MouseButton::Left) => 3,
                    helix_view::input::MouseEventKind::Up(MouseButton::Right) => 4,
                    helix_view::input::MouseEventKind::Up(MouseButton::Middle) => 5,
                    helix_view::input::MouseEventKind::Drag(MouseButton::Left) => 6,
                    helix_view::input::MouseEventKind::Drag(MouseButton::Right) => 7,
                    helix_view::input::MouseEventKind::Drag(MouseButton::Middle) => 8,
                    helix_view::input::MouseEventKind::Moved => 9,
                    helix_view::input::MouseEventKind::ScrollDown => 10,
                    helix_view::input::MouseEventKind::ScrollUp => 11,
                    helix_view::input::MouseEventKind::ScrollLeft => 12,
                    helix_view::input::MouseEventKind::ScrollRight => 13,
                }
                .into_steelval()
            } else {
                false.into_steelval()
            }
        })
        .register_fn("event-mouse-row", |event: Event| {
            if let Event::Mouse(MouseEvent { row, .. }) = event {
                row.into_steelval()
            } else {
                false.into_steelval()
            }
        })
        .register_fn("event-mouse-col", |event: Event| {
            if let Event::Mouse(MouseEvent { column, .. }) = event {
                column.into_steelval()
            } else {
                false.into_steelval()
            }
        })
        // Is this mouse event within the area provided
        .register_fn("mouse-event-within-area?", |event: Event, area: Rect| {
            if let Event::Mouse(MouseEvent { row, column, .. }) = event {
                column > area.x
                    && column < area.x + area.width
                    && row > area.y
                    && row < area.y + area.height
            } else {
                false
            }
        });

    macro_rules! register {
        ($name:expr, $function:expr, $doc:expr) => {
            module.register_fn($name, $function);
            {
                let doc = format_docstring($doc);
                builtin_components_module.push_str(&format!(
                    r#"
(provide {})
;;@doc
{}
(define {} helix.components.{})
                    "#,
                    $name, doc, $name, $name
                ));
            }
        };
    }

    macro_rules! register_key_events {
        ($ ( $name:expr => $key:tt ) , *, ) => {
            $(
              register!(concat!("key-event-", $name, "?"), |event: Event| {
                  matches!(
                      event,
                      Event::Key(
                          KeyEvent {
                              code: KeyCode::$key,
                              ..
                          }
                      ))
                  },
                &format!(r#"
Check whether the given event is the key: {}

```scheme
(key-event-{}? event)
```
event: Event?"#, $name, $name));
            )*
        };
    }

    // Key events for individual key codes
    register_key_events!(
        "escape" => Esc,
        "backspace" => Backspace,
        "enter" => Enter,
        "left" => Left,
        "right" => Right,
        "up" => Up,
        "down" => Down,
        "home" => Home,
        "end" => End,
        "page-up" => PageUp,
        "page-down" => PageDown,
        "tab" => Tab,
        "delete" => Delete,
        "insert" => Insert,
        "null" => Null,
        "caps-lock" => CapsLock,
        "scroll-lock" => ScrollLock,
        "num-lock" => NumLock,
        "print-screen" => PrintScreen,
        "pause" => Pause,
        "menu" => Menu,
        "keypad-begin" => KeypadBegin,
    );

    if generate_sources {
        generate_module("components.scm", &builtin_components_module);
        configure_lsp_builtins("component", &module);
    }

    module
}

fn buffer_set_string(
    buffer: &mut tui::buffer::Buffer,
    x: u16,
    y: u16,
    string: SteelVal,
    style: Style,
) -> steel::rvals::Result<()> {
    match string {
        SteelVal::StringV(string) => {
            buffer.set_string(x, y, string.as_str(), style);
            Ok(())
        }
        SteelVal::Custom(c) => {
            if let Some(string) =
                as_underlying_type::<steel::steel_vm::ffi::MutableString>(c.read().as_ref())
            {
                buffer.set_string(x, y, string.string.as_str(), style);
                Ok(())
            } else {
                steel::stop!(TypeMismatch => "buffer-set-string! expected a string")
            }
        }
        _ => {
            steel::stop!(TypeMismatch => "buffer-set-string! expected a string")
        }
    }
}

/// A dynamic component, used for rendering
pub struct SteelDynamicComponent {
    // TODO: currently the component id requires using a &'static str,
    // however in a world with dynamic components that might not be
    // the case anymore
    name: String,
    // This _should_ be a struct, but in theory can be whatever you want. It will be the first argument
    // passed to the functions in the remainder of the struct.
    state: SteelVal,
    handle_event: Option<SteelVal>,
    _should_update: Option<SteelVal>,
    render: SteelVal,
    cursor: Option<SteelVal>,
    required_size: Option<SteelVal>,

    // Cached key event; we keep this around so that when sending
    // events to the event handler, we can reuse the heap allocation
    // instead of re-allocating for every event (which might be a lot)
    key_event: Option<SteelVal>,

    // Just root all of the inputs so that we don't have any issues with
    // things dropping
    _roots: Vec<RootedSteelVal>,

    generation: usize,
}

impl SteelDynamicComponent {
    pub fn new(
        name: String,
        state: SteelVal,
        render: SteelVal,
        h: HashMap<String, SteelVal>,
    ) -> Self {
        let mut roots = vec![state.clone().as_rooted(), render.clone().as_rooted()];

        for value in h.values() {
            roots.push(value.clone().as_rooted());
        }

        // Keep root tokens around? Otherwise we're not going to be
        // able to reach these values from the runtime.
        Self {
            name,
            state,
            render,
            handle_event: h.get("handle_event").cloned(),
            _should_update: h.get("should_update").cloned(),
            cursor: h.get("cursor").cloned(),
            required_size: h.get("required_size").cloned(),
            key_event: None,
            _roots: roots,
            generation: load_generation(),
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
}

impl Custom for SteelDynamicComponent {}

impl Custom for Box<dyn Component> {}

#[derive(Clone)]
enum SteelEventResult {
    Consumed,
    Ignored,
    IgnoreAndClose,
    Close,
    ConsumedWithoutRerender,
}

impl Custom for SteelEventResult {}

impl Component for SteelDynamicComponent {
    fn name(&self) -> Option<&str> {
        Some(&self.name)
    }

    fn render(
        &mut self,
        area: helix_view::graphics::Rect,
        frame: &mut tui::buffer::Buffer,
        ctx: &mut compositor::Context,
    ) {
        if !is_current_generation(self.generation) {
            return;
        }

        // Skip rendering if the function is actually false
        if let SteelVal::BoolV(false) = self.render {
            return;
        }

        let mut ctx = with_context_guard(ctx);

        // Pass the `state` object through - this can be used for storing the state of whatever plugin thing we're
        // attempting to render
        let thunk = |engine: &mut Engine, f| {
            engine.call_function_with_args_from_mut_slice(
                self.render.clone(),
                &mut [self.state.clone(), area.into_steelval().unwrap(), f],
            )
        };

        enter_engine(|guard| {
            if let Err(e) = guard
                .with_mut_reference::<tui::buffer::Buffer, tui::buffer::Buffer>(frame)
                .with_mut_reference::<Context, Context>(&mut ctx)
                .consume(|engine, args| {
                    let mut arg_iter = args.into_iter();

                    let buffer = arg_iter.next().unwrap();
                    let context = arg_iter.next().unwrap();

                    engine.update_value(CTX, context);

                    (thunk)(engine, buffer)
                })
            {
                let name = self.name.clone();
                super::present_error_inside_engine_context_with_callback(
                    &mut ctx,
                    guard,
                    e,
                    move |compositor| {
                        compositor.remove_by_dynamic_name(&name);
                    },
                );
            }
        });
    }

    // TODO: Pass in event as well? Need to have immutable reference type
    // Otherwise, we're gonna be in a bad spot. For now - just clone the object and pass it through.
    // Clong is _not_ ideal, but it might be all we can do for now.
    fn handle_event(
        &mut self,
        event: &Event,
        ctx: &mut compositor::Context,
    ) -> compositor::EventResult {
        // Ignore this event off the stack
        if !is_current_generation(self.generation) {
            return compositor::EventResult::Ignored(Some(Box::new(
                |compositor: &mut compositor::Compositor, _| {
                    compositor.pop();
                },
            )));
        }

        if let Some(handle_event) = &mut self.handle_event {
            let mut ctx = with_context_guard(ctx);

            match self.key_event.as_mut() {
                Some(SteelVal::Custom(key_event)) => {
                    // Save the headache, reuse the allocation
                    if let Some(inner) =
                        steel::rvals::as_underlying_type_mut::<Event>(key_event.write().as_mut())
                    {
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
                engine.call_function_with_args_from_mut_slice(
                    handle_event.clone(),
                    &mut [self.state.clone(), self.key_event.clone().unwrap()],
                )
            };

            let res = match enter_engine(|guard| {
                guard
                    .with_mut_reference::<Context, Context>(&mut ctx)
                    .consume_once(move |engine, arguments| {
                        let context = arguments.into_iter().next().unwrap();
                        engine.update_value(CTX, context);
                        thunk(engine)
                    })
            }) {
                Ok(v) => {
                    let value = SteelEventResult::from_steelval(&v);

                    match value {
                        Ok(SteelEventResult::Close) => compositor::EventResult::Consumed(Some(
                            Box::new(|compositor: &mut compositor::Compositor, _| {
                                // remove the layer
                                compositor.pop();
                            }),
                        )),
                        Ok(SteelEventResult::Consumed) => compositor::EventResult::Consumed(None),
                        Ok(SteelEventResult::ConsumedWithoutRerender) => {
                            compositor::EventResult::ConsumedWithoutRerender
                        }
                        Ok(SteelEventResult::Ignored) => compositor::EventResult::Ignored(None),
                        Ok(SteelEventResult::IgnoreAndClose) => compositor::EventResult::Ignored(
                            Some(Box::new(|compositor: &mut compositor::Compositor, _| {
                                // remove the layer
                                compositor.pop();
                            })),
                        ),
                        _ => compositor::EventResult::Ignored(None),
                    }
                }
                Err(e) => {
                    // Present the error
                    enter_engine(|x| present_error_inside_engine_context(&mut ctx, x, e));

                    compositor::EventResult::Ignored(None)
                }
            };

            res
        } else {
            compositor::EventResult::Ignored(None)
        }
    }

    fn should_update(&self) -> bool {
        true
    }

    fn cursor(
        &self,
        area: helix_view::graphics::Rect,
        _ctx: &Editor,
    ) -> (
        Option<helix_core::Position>,
        helix_view::graphics::CursorKind,
    ) {
        if !is_current_generation(self.generation) {
            return (None, helix_view::graphics::CursorKind::Hidden);
        }

        if let Some(cursor) = &self.cursor {
            // Pass the `state` object through - this can be used for storing the state of whatever plugin thing we're
            // attempting to render
            let thunk = |engine: &mut Engine| {
                engine.call_function_with_args_from_mut_slice(
                    cursor.clone(),
                    &mut [self.state.clone(), area.into_steelval().unwrap()],
                )
            };

            let cursor_call_result = enter_engine(thunk);

            match cursor_call_result {
                Ok(c) => match c {
                    // Specify the style of the list
                    SteelVal::ListV(generic_list) => {
                        if generic_list.len() != 2 {
                            log::info!("Error: Unable to destructure list of length: {} while setting the cursor position", generic_list.len());
                            (None, CursorKind::Block)
                        } else {
                            let maybe_position = Option::<helix_core::Position>::from_steelval(
                                generic_list.get(0).unwrap(),
                            );

                            let cursor_kind = match generic_list.get(1) {
                                Some(SteelVal::SymbolV(s) | SteelVal::StringV(s)) => {
                                    match s.as_str() {
                                        "block" => CursorKind::Block,
                                        "hidden" => CursorKind::Hidden,
                                        "bar" => CursorKind::Bar,
                                        "underline" => CursorKind::Underline,
                                        _ => CursorKind::Block,
                                    }
                                }

                                _ => CursorKind::Block,
                            };

                            match maybe_position {
                                Ok(v) => (v, cursor_kind),
                                Err(e) => {
                                    log::info!("Error: {:?}", e);
                                    (None, cursor_kind)
                                }
                            }
                        }
                    }
                    other => {
                        let result = Option::<helix_core::Position>::from_steelval(&other);
                        match result {
                            Ok(v) => (v, CursorKind::Block),
                            // TODO: Figure out how to pop up an error message
                            Err(_e) => {
                                log::info!("Error: {:?}", _e);
                                (None, CursorKind::Block)
                            }
                        }
                    }
                },
                Err(e) => {
                    log::info!("Error: {:?}", e);
                    (None, CursorKind::Block)
                }
            }
        } else {
            (None, helix_view::graphics::CursorKind::Hidden)
        }
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        if !is_current_generation(self.generation) {
            return None;
        }

        if let Some(required_size) = &mut self.required_size {
            enter_engine(|x| {
                x.call_function_with_args_from_mut_slice(
                    required_size.clone(),
                    &mut [self.state.clone(), viewport.into_steelval().unwrap()],
                )
            })
            .and_then(|x| Option::<(u16, u16)>::from_steelval(&x))
            .ok()
            .flatten()
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
