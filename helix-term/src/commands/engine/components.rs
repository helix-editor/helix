use std::{collections::HashMap, sync::Arc};

use helix_core::Position;
use helix_view::{
    graphics::{Color, CursorKind, Rect, UnderlineStyle},
    input::{Event, KeyEvent, MouseButton, MouseEvent},
    keyboard::{KeyCode, KeyModifiers},
    theme::{Modifier, Style},
    Editor,
};
use steel::{
    rvals::{
        as_underlying_type, AsRefSteelVal, AsRefSteelValFromRef, Custom, FromSteelVal,
        IntoSteelVal, SteelString,
    },
    steel_vm::{builtin::BuiltInModule, engine::Engine, register_fn::RegisterFn},
    SteelVal,
};
use tokio::sync::Mutex;
use tui::{
    buffer::Buffer,
    text::Text,
    widgets::{self, Block, BorderType, Borders, ListItem, Widget},
};

use crate::{
    commands::{engine::steel::BoxDynComponent, Context},
    compositor::{self, Component},
    ui::overlay::overlaid,
};

use super::steel::{
    enter_engine, format_docstring, present_error_inside_engine_context, WrappedDynComponent,
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
        if let Err(_) = self.channel.send(String::from_utf8_lossy(buf).to_string()) {
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

    let mut builtin_components_module = if generate_sources {
        "(require-builtin helix/components as helix.components.)".to_string()
    } else {
        String::new()
    };

    macro_rules! register {
        (value, $name:expr, $function:expr, $doc:expr) => {
            module.register_value($name, $function);
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

        (value, $name:expr, $function:expr) => {
            module.register_value($name, $function);
            {
                builtin_components_module.push_str(&format!(
                    r#"
(provide {})
(define {} helix.components.{})
                    "#,
                    $name, $name, $name
                ));
            }
        };

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

        ($name:expr, $function:expr) => {
            module.register_fn($name, $function);
            {
                builtin_components_module.push_str(&format!(
                    r#"
(provide {})
(define {} helix.components.{})
                    "#,
                    $name, $name, $name
                ));
            }
        };

        (ctx, $name:expr, $function:expr, $arity:expr, $doc:expr) => {
            module.register_fn($name, $function);
            let mut function_expr = Vec::with_capacity($arity);
            for arg in 0..$arity {
                function_expr.push(format!("arg{}", arg));
            }

            let formatted = function_expr.join(" ");

            {
                let doc = format_docstring($doc);
                builtin_components_module.push_str(&format!(
                    r#"
(provide {})
;;@doc
{}
(define ({} {}) (helix.components.{} {}))
                    "#,
                    $name, doc, $name, &formatted, $name, &formatted
                ));
            }
        };
    }

    register!("async-read-line", AsyncReader::read_line);
    register!("make-async-reader-writer", || {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        let writer = AsyncWriter { channel: sender };
        let reader = AsyncReader {
            channel: Arc::new(Mutex::new(receiver)),
        };

        vec![
            SteelVal::new_dyn_writer_port(writer),
            reader.into_steelval().unwrap(),
        ]
    });
    register!(
        "theme->bg",
        |ctx: &mut Context| { ctx.editor.theme.get("ui.background") },
        "Gets the `Style` associated with the bg for the current theme"
    );
    register!(
        "theme->fg",
        |ctx: &mut Context| { ctx.editor.theme.get("ui.text") },
        "Gets the `style` associated with the fg for the current theme"
    );
    register!(
        ctx,
        "theme-scope",
        |ctx: &mut Context, scope: SteelString| {
            ctx.editor.theme.get(scope.as_str());
        },
        1,
        "Get the `Style` associated with the given scope from the current theme"
    );

    register!(
        "Position?",
        |position: SteelVal| { Position::as_ref(&position).is_ok() },
        r#"Check if the given value is a `Position`

(Position? value) -> bool?

value : any?

        "#
    );

    register!(
        "Style?",
        |style: SteelVal| Style::as_ref(&style).is_ok(),
        r#"Check if the given valuie is `Style`

(Style? value) -> bool?

value : any?
"#
    );

    register!(
        "Buffer?",
        |value: SteelVal| { Buffer::as_ref_from_ref(&value).is_ok() },
        r#"
Checks if the given value is a `Buffer`

(Buffer? value) -> bool?

value : any?
        "#
    );

    register!(
        "buffer-area",
        |buffer: &mut Buffer| buffer.area,
        r#"
Get the `Rect` associated with the given `Buffer`

(buffer-area buffer)

* buffer : Buffer?
        "#
    );

    register!(
        "frame-set-string!",
        buffer_set_string,
        r#"
Set the string at the given `x` and `y` positions for the given `Buffer`, with a provided `Style`.

(frame-set-string! buffer x y string style)

    buffer : Buffer?,
    x : int?,
    y : int?,
    string: string?,
    style: Style?,
        "#
    );

    register!("new-component!", SteelDynamicComponent::new_dyn);

    register!(
        "position",
        Position::new,
        r#"
Construct a new `Position`.

(position row col) -> Position?

row : int?
col : int?
        "#
    );
    register!(
        "position-row",
        |position: &Position| position.row,
        r#"
Get the row associated with the given `Position`.

(position-row pos) -> int?

pos : `Position?`
        "#
    );
    register!(
        "position-col",
        |position: &Position| position.col,
        r#"
Get the col associated with the given `Position`.

(position-col pos) -> int?

pos : `Position?`
"#
    );

    register!(
        "set-position-row!",
        |position: &mut Position, row: usize| {
            position.row = row;
        },
        r#"Set the row for the given `Position`
(set-position-row! pos row)

pos : Position?
row : int?
        "#
    );
    register!(
        "set-position-col!",
        |position: &mut Position, col: usize| {
            position.col = col;
        },
        r#"Set the col for the given `Position`

(set-position-col! pos col)

pos : Position?
col : int?
        "#
    );

    register!(
        "area",
        helix_view::graphics::Rect::new,
        r#"
Constructs a new `Rect`.

(area x y width height)

* x : int?
* y : int?
* width: int?
* height: int?

# Examples

```scheme
(area 0 0 100 200)
```
"#
    );
    register!(
        "area-x",
        |area: &helix_view::graphics::Rect| area.x,
        "Get the `x` value of the given `Rect`"
    );
    register!(
        "area-y",
        |area: &helix_view::graphics::Rect| area.y,
        "Get the `y` value of the given `Rect`"
    );
    register!(
        "area-width",
        |area: &helix_view::graphics::Rect| area.width,
        "Get the `width` value of the given `Rect`"
    );
    register!(
        "area-height",
        |area: &helix_view::graphics::Rect| { area.height },
        "Get the `height` value of the given `Rect`"
    );

    register!("overlaid", |component: &mut WrappedDynComponent| {
        let inner: Option<Box<dyn Component + Send + Sync + 'static>> =
            component.inner.take().map(|x| {
                Box::new(overlaid(BoxDynComponent::new(x)))
                    as Box<dyn Component + Send + Sync + 'static>
            });

        component.inner = inner;
    });
    register!("widget/list", |items: Vec<String>| {
        widgets::List::new(
            items
                .into_iter()
                .map(|x| ListItem::new(Text::from(x)))
                .collect::<Vec<_>>(),
        )
    });

    register!(
        "widget/list/render",
        |buf: &mut Buffer, area: Rect, list: widgets::List| list.render(area, buf)
    );
    register!("block", || {
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(Color::Black))
    });

    register!(
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
        r#"
Create a `Block` with the provided styling, borders, and border type.

(make-block style border-style borders border_type)

* style : Style?
* border-style : Style?
* borders : string?
* border-type: String?

Valid border-types include:
* "plain"
* "rounded"
* "double"
* "thick"

Valid borders include:
* "top"
* "left"
* "right"
* "bottom"
* "all"
        "#
    );

    register!(
        "block/render",
        |buf: &mut Buffer, area: Rect, block: Block| block.render(area, buf)
    );
    register!(
        "buffer/clear",
        Buffer::clear,
        "Clear a `Rect` in the `Buffer`"
    );

    register!(
        "buffer/clear-with",
        Buffer::clear_with,
        "Clear a `Rect` in the `Buffer` with a default `Style`"
    );

    // Mutate a color in place, to save some headache.
    register!("set-color-rgb!", |color: &mut Color,
                                 r: u8,
                                 g: u8,
                                 b: u8| {
        *color = Color::Rgb(r, g, b);
    });

    register!("set-color-indexed!", |color: &mut Color, index: u8| {
        *color = Color::Indexed(index);
    });

    register!(
        "Color?",
        |color: SteelVal| { Color::as_ref(&color).is_ok() },
        r#"Check if the given value is a `Color`.

(Color? value) -> bool?

value : any?

        "#
    );

    register!(value, "Color/Reset", Color::Reset.into_steelval().unwrap());
    register!(value, "Color/Black", Color::Black.into_steelval().unwrap());
    register!(value, "Color/Red", Color::Red.into_steelval().unwrap());
    register!(value, "Color/White", Color::White.into_steelval().unwrap());
    register!(value, "Color/Green", Color::Green.into_steelval().unwrap());
    register!(
        value,
        "Color/Yellow",
        Color::Yellow.into_steelval().unwrap()
    );
    register!(value, "Color/Blue", Color::Blue.into_steelval().unwrap());
    register!(
        value,
        "Color/Magenta",
        Color::Magenta.into_steelval().unwrap()
    );
    register!(value, "Color/Cyan", Color::Cyan.into_steelval().unwrap());
    register!(value, "Color/Gray", Color::Gray.into_steelval().unwrap());
    register!(
        value,
        "Color/LightRed",
        Color::LightRed.into_steelval().unwrap()
    );

    register!(
        value,
        "Color/LightGreen",
        Color::LightGreen.into_steelval().unwrap()
    );
    register!(
        value,
        "Color/LightYellow",
        Color::LightYellow.into_steelval().unwrap()
    );
    register!(
        value,
        "Color/LightBlue",
        Color::LightBlue.into_steelval().unwrap()
    );
    register!(
        value,
        "Color/LightMagenta",
        Color::LightMagenta.into_steelval().unwrap()
    );
    register!(
        value,
        "Color/LightCyan",
        Color::LightCyan.into_steelval().unwrap()
    );
    register!(
        value,
        "Color/LightGray",
        Color::LightGray.into_steelval().unwrap()
    );

    register!("Color/rgb", Color::Rgb);
    register!("Color-red", Color::red);
    register!("Color-green", Color::green);
    register!("Color-blue", Color::blue);
    register!("Color/Indexed", Color::Indexed);

    register!("set-style-fg!", |style: &mut Style, color: Color| {
        style.fg = Some(color);
    });
    register!("style-fg", Style::fg);
    register!("style-bg", Style::bg);
    register!("style-with-italics", |style: &Style| {
        let patch = Style::default().add_modifier(Modifier::ITALIC);
        style.patch(patch)
    });
    register!("style-with-bold", |style: Style| {
        let patch = Style::default().add_modifier(Modifier::BOLD);
        style.patch(patch)
    });
    register!("style-with-dim", |style: &Style| {
        let patch = Style::default().add_modifier(Modifier::DIM);
        style.patch(patch)
    });
    register!("style-with-slow-blink", |style: Style| {
        let patch = Style::default().add_modifier(Modifier::SLOW_BLINK);
        style.patch(patch)
    });
    register!("style-with-rapid-blink", |style: Style| {
        let patch = Style::default().add_modifier(Modifier::RAPID_BLINK);
        style.patch(patch)
    });
    register!("style-with-reversed", |style: Style| {
        let patch = Style::default().add_modifier(Modifier::REVERSED);
        style.patch(patch)
    });
    register!("style-with-hidden", |style: Style| {
        let patch = Style::default().add_modifier(Modifier::HIDDEN);
        style.patch(patch)
    });
    register!("style-with-crossed-out", |style: Style| {
        let patch = Style::default().add_modifier(Modifier::CROSSED_OUT);
        style.patch(patch)
    });
    register!("style->fg", |style: &Style| style.fg);
    register!("style->bg", |style: &Style| style.bg);
    register!("set-style-bg!", |style: &mut Style, color: Color| {
        style.bg = Some(color);
    });
    register!("style-underline-color", Style::underline_color);
    register!("style-underline-style", Style::underline_style);

    register!(
        value,
        "Underline/Reset",
        UnderlineStyle::Reset.into_steelval().unwrap()
    );
    register!(
        value,
        "Underline/Line",
        UnderlineStyle::Line.into_steelval().unwrap()
    );
    register!(
        value,
        "Underline/Curl",
        UnderlineStyle::Curl.into_steelval().unwrap()
    );
    register!(
        value,
        "Underline/Dotted",
        UnderlineStyle::Dotted.into_steelval().unwrap()
    );
    register!(
        value,
        "Underline/Dashed",
        UnderlineStyle::Dashed.into_steelval().unwrap()
    );
    register!(
        value,
        "Underline/DoubleLine",
        UnderlineStyle::DoubleLine.into_steelval().unwrap()
    );
    register!(
        value,
        "event-result/consume",
        SteelEventResult::Consumed.into_steelval().unwrap()
    );
    register!(
        value,
        "event-result/consume-without-rerender",
        SteelEventResult::ConsumedWithoutRerender
            .into_steelval()
            .unwrap()
    );
    register!(
        value,
        "event-result/ignore",
        SteelEventResult::Ignored.into_steelval().unwrap()
    );
    register!(
        value,
        "event-result/close",
        SteelEventResult::Close.into_steelval().unwrap()
    );
    register!("style", || Style::default());

    register!("key-event-char", |event: Event| {
        if let Event::Key(event) = event {
            event.char()
        } else {
            None
        }
    });
    register!("key-event-modifier", |event: Event| {
        if let Event::Key(KeyEvent { modifiers, .. }) = event {
            Some(modifiers.bits())
        } else {
            None
        }
    });

    register!(
        value,
        "key-modifier-ctrl",
        SteelVal::IntV(KeyModifiers::CONTROL.bits() as isize)
    );
    register!(
        value,
        "key-modifier-shift",
        SteelVal::IntV(KeyModifiers::SHIFT.bits() as isize)
    );
    register!(
        value,
        "key-modifier-alt",
        SteelVal::IntV(KeyModifiers::ALT.bits() as isize)
    );

    register!("key-event-F?", |event: Event, number: u8| match event {
        Event::Key(KeyEvent {
            code: KeyCode::F(x),
            ..
        }) if number == x => true,
        _ => false,
    });
    register!("mouse-event?", |event: Event| {
        matches!(event, Event::Mouse(_))
    });
    register!("event-mouse-kind", |event: Event| {
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
    });
    register!("event-mouse-row", |event: Event| {
        if let Event::Mouse(MouseEvent { row, .. }) = event {
            row.into_steelval()
        } else {
            false.into_steelval()
        }
    });
    register!("event-mouse-col", |event: Event| {
        if let Event::Mouse(MouseEvent { column, .. }) = event {
            column.into_steelval()
        } else {
            false.into_steelval()
        }
    });
    // Is this mouse event within the area provided
    register!("mouse-event-within-area?", |event: Event, area: Rect| {
        if let Event::Mouse(MouseEvent { row, column, .. }) = event {
            column > area.x
                && column < area.x + area.width
                && row > area.y
                && row < area.y + area.height
        } else {
            false
        }
    });

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
                  });
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
        if let Some(mut target_directory) =
            crate::commands::engine::steel::alternative_runtime_search_path()
        {
            if !target_directory.exists() {
                std::fs::create_dir_all(&target_directory).unwrap();
            }
            target_directory.push("components.scm");
            std::fs::write(target_directory, &builtin_components_module).unwrap();
        }
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

    // buffer.set_string(x, y, string.as_str(), style)
}

/// A dynamic component, used for rendering
#[derive(Clone)]
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
}

impl SteelDynamicComponent {
    pub fn new(
        name: String,
        state: SteelVal,
        render: SteelVal,
        h: HashMap<String, SteelVal>,
    ) -> Self {
        Self {
            name,
            state,
            render,
            handle_event: h.get("handle_event").cloned(),
            _should_update: h.get("should_update").cloned(),
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
}

impl Custom for SteelDynamicComponent {}

impl Custom for Box<dyn Component> {}

#[derive(Clone)]
enum SteelEventResult {
    Consumed,
    Ignored,
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

                    engine.update_value("*helix.cx*", context);

                    (thunk)(engine, buffer)
                })
            {
                present_error_inside_engine_context(&mut ctx, guard, e)
            }
        })
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

            let close_fn = compositor::EventResult::Consumed(Some(Box::new(
                |compositor: &mut compositor::Compositor, _| {
                    // remove the layer
                    compositor.pop();
                },
            )));

            // let event = match event {
            //     Event::Key(event) => *event,
            //     _ => return compositor::EventResult::Ignored(None),
            // };

            match enter_engine(|guard| {
                guard
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
                        Ok(SteelEventResult::ConsumedWithoutRerender) => {
                            compositor::EventResult::ConsumedWithoutRerender
                        }
                        Ok(SteelEventResult::Ignored) => compositor::EventResult::Ignored(None),
                        _ => match event {
                            // ctrl!('c') | key!(Esc) => close_fn,
                            _ => compositor::EventResult::Ignored(None),
                        },
                    }
                }
                Err(e) => {
                    // Present the error
                    enter_engine(|x| present_error_inside_engine_context(&mut ctx, x, e));

                    compositor::EventResult::Ignored(None)
                }
            }
        } else {
            compositor::EventResult::Ignored(None)
        }
    }

    fn should_update(&self) -> bool {
        true

        // if let Some(should_update) = &self.should_update {
        //     match ENGINE.with(|x| {
        //         let res = x
        //             .borrow_mut()
        //             .call_function_with_args(should_update.clone(), vec![self.state.clone()]);

        //         res
        //     }) {
        //         Ok(v) => bool::from_steelval(&v).unwrap_or(true),
        //         Err(_) => true,
        //     }
        // } else {
        //     true
        // }
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
        if let Some(cursor) = &self.cursor {
            // Pass the `state` object through - this can be used for storing the state of whatever plugin thing we're
            // attempting to render
            let thunk = |engine: &mut Engine| {
                engine.call_function_with_args_from_mut_slice(
                    cursor.clone(),
                    &mut [self.state.clone(), area.into_steelval().unwrap()],
                )
            };

            let result =
                Option::<helix_core::Position>::from_steelval(&enter_engine(|x| thunk(x).unwrap()));

            match result {
                Ok(v) => (v, CursorKind::Block),
                // TODO: Figure out how to pop up an error message
                Err(_e) => {
                    log::info!("Error: {:?}", _e);
                    (None, CursorKind::Block)
                }
            }
        } else {
            (None, helix_view::graphics::CursorKind::Hidden)
        }
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        // let name = self.type_name();

        if let Some(required_size) = &mut self.required_size {
            // log::info!("Calling required-size inside: {}", name);

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
            match enter_engine(|x| {
                x.call_function_with_args_from_mut_slice(
                    required_size.clone(),
                    &mut [self.state.clone(), viewport.into_steelval().unwrap()],
                )
            })
            .and_then(|x| Option::<(u16, u16)>::from_steelval(&x))
            {
                Ok(v) => v,
                // TODO: Figure out how to present an error
                Err(_e) => None,
            }
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
