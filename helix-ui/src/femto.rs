use resource::resource;

use instant::Instant;
use winit::event::{ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
//use glutin::{GlRequest, Api};

use femtovg::{
    //CompositeOperation,
    renderer::OpenGl,
    Align,
    Baseline,
    Canvas,
    Color,
    FontId,
    ImageFlags,
    Paint,
    Path,
    Renderer,
    Solidity,
};

// mezzopiano
//  —
// 03/13/2022
// I'm also assuming that there's some logic bugs in the demo application, which wasn't built with this in mind; I have a much simpler application that I'm happy to show if that would be helpful (would need to extract an example). As a rough solution, I apply the following transformation on Winit's WindowEvent::ScaleFactorChanged:

//     /* ... in an application struct/impl ... */
//     pub fn rescale(
//         &mut self,
//         new_size: winit::dpi::PhysicalSize<u32>,
//         new_scale_factor: f64,
//     ) {
//         // Update translation
//         // (TODO: This is a guestimate and not well-tested;
//         // the size updates might turn out ot be completely unnecessary)
//         let shift = self.size.height as f32 - new_size.height as f32;
//         self.canvas.translate(0.0, -shift);

//         // Update properties
//         self.size = new_size;
//         self.scale_factor = new_scale_factor;
//         self.canvas.set_size(
//             self.size.width,
//             self.size.height,
//             self.scale_factor as f32,
//         )
//     }

// With this, the canvas position and scale is preserved while the window is moved across screens, but as I'd like to apply further translations and keeping track of everything is getting very hard 😅 . I'm wondering if I'm making a mistake somewhere, or if there might be some way to do this in femto.

pub fn quantize(a: f32, d: f32) -> f32 {
    (a / d + 0.5).trunc() * d
}

struct Fonts {
    regular: FontId,
    bold: FontId,
    icons: FontId,
}

fn main() {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(all(debug_assertions, target_arch = "wasm32"))]
    console_error_panic_hook::set_once();

    let el = EventLoop::new();

    #[cfg(not(target_arch = "wasm32"))]
    let (renderer, windowed_context) = {
        use glutin::ContextBuilder;

        let wb = WindowBuilder::new()
            .with_inner_size(winit::dpi::LogicalSize::<f32>::new(1000., 600.))
            .with_title("femtovg demo");

        //let windowed_context = ContextBuilder::new().with_gl(GlRequest::Specific(Api::OpenGlEs, (2, 0))).with_vsync(false).build_windowed(wb, &el).unwrap();
        //let windowed_context = ContextBuilder::new().with_vsync(false).with_multisampling(8).build_windowed(wb, &el).unwrap();
        let windowed_context = ContextBuilder::new()
            .with_vsync(true) // TODO: set to true?
            .build_windowed(wb, &el)
            .unwrap();
        let windowed_context = unsafe { windowed_context.make_current().unwrap() };

        let renderer =
            OpenGl::new_from_glutin_context(&windowed_context).expect("Cannot create renderer");

        (renderer, windowed_context)
    };

    #[cfg(target_arch = "wasm32")]
    let (renderer, window) = {
        use wasm_bindgen::JsCast;

        let canvas = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();

        use winit::platform::web::WindowBuilderExtWebSys;

        let renderer = OpenGl::new_from_html_canvas(&canvas).expect("Cannot create renderer");

        let window = WindowBuilder::new()
            .with_canvas(Some(canvas))
            .build(&el)
            .unwrap();

        (renderer, window)
    };

    let mut canvas = Canvas::new(renderer).expect("Cannot create canvas");

    // TODO: better femtovg support for variable fonts
    let fonts = Fonts {
        regular: canvas
            .add_font_mem(&resource!("assets/fonts/Inter\ Variable/Inter.ttf"))
            .expect("Cannot add font"),
        bold: canvas
            .add_font_mem(&resource!("assets/fonts/Inter Variable/Inter.ttf"))
            .expect("Cannot add font"),
        icons: canvas
            .add_font_mem(&resource!("assets/entypo.ttf"))
            .expect("Cannot add font"),
    };

    //canvas.add_font("/usr/share/fonts/noto/NotoSansArabic-Regular.ttf").expect("Cannot add font");

    //let image_id = canvas.create_image_file("assets/RoomRender.jpg", ImageFlags::FLIP_Y).expect("Cannot create image");
    //canvas.blur_image(image_id, 10, 1050, 710, 200, 200);

    //let image_id = canvas.load_image_file("assets/RoomRender.jpg", ImageFlags::FLIP_Y).expect("Cannot create image");

    // let images = vec![
    //     canvas
    //         .load_image_mem(&resource!("assets/images/image1.jpg"), ImageFlags::empty())
    //         .unwrap(),
    //     canvas
    //         .load_image_mem(&resource!("assets/images/image2.jpg"), ImageFlags::empty())
    //         .unwrap(),
    // ];

    let mut screenshot_image_id = None;

    let start = Instant::now();
    let mut prevt = start;

    let mut mousex = 0.0;
    let mut mousey = 0.0;
    let mut dragging = false;

    let mut perf = PerfGraph::new();

    el.run(move |event, _, control_flow| {
        #[cfg(not(target_arch = "wasm32"))]
        let window = windowed_context.window();

        *control_flow = ControlFlow::Poll;

        match event {
            Event::LoopDestroyed => return,
            Event::WindowEvent { ref event, .. } => match event {
                #[cfg(not(target_arch = "wasm32"))]
                WindowEvent::Resized(physical_size) => {
                    println!("resized!");
                    // TODO: use DPI here?
                    windowed_context.resize(*physical_size);
                }
                WindowEvent::CursorMoved {
                    device_id: _,
                    position,
                    ..
                } => {
                    if dragging {
                        let p0 = canvas
                            .transform()
                            .inversed()
                            .transform_point(mousex, mousey);
                        let p1 = canvas
                            .transform()
                            .inversed()
                            .transform_point(position.x as f32, position.y as f32);

                        canvas.translate(p1.0 - p0.0, p1.1 - p0.1);
                    }

                    mousex = position.x as f32;
                    mousey = position.y as f32;
                }
                WindowEvent::MouseWheel {
                    device_id: _,
                    delta,
                    ..
                } => match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => {
                        let pt = canvas
                            .transform()
                            .inversed()
                            .transform_point(mousex, mousey);
                        canvas.translate(pt.0, pt.1);
                        canvas.scale(1.0 + (y / 10.0), 1.0 + (y / 10.0));
                        canvas.translate(-pt.0, -pt.1);
                    }

                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        let y = pos.y as f32;
                        let pt = canvas
                            .transform()
                            .inversed()
                            .transform_point(mousex, mousey);
                        let rate = 2000.0;
                        canvas.translate(pt.0, pt.1);
                        canvas.scale(1.0 + (y / rate), 1.0 + (y / rate));
                        canvas.translate(-pt.0, -pt.1);
                    }
                },
                WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    state,
                    ..
                } => match state {
                    ElementState::Pressed => dragging = true,
                    ElementState::Released => dragging = false,
                },
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::S),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    if let Some(screenshot_image_id) = screenshot_image_id {
                        canvas.delete_image(screenshot_image_id);
                    }

                    if let Ok(image) = canvas.screenshot() {
                        screenshot_image_id = Some(
                            canvas
                                .create_image(image.as_ref(), ImageFlags::empty())
                                .unwrap(),
                        );
                    }
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => (),
            },
            Event::RedrawRequested(_) => {
                let now = Instant::now();
                let dt = (now - prevt).as_secs_f32();
                prevt = now;

                perf.update(dt);

                let dpi_factor = window.scale_factor();
                // println!("DPI {}", dpi_factor);
                // let dpi_factor = 0.5f64;
                let size = window.inner_size();
                // let size: winit::dpi::LogicalSize<u32> = window.inner_size().to_logical(dpi_factor); // TODO: adjust for dpi
                // window.set_inner_size(size);
                // let size = window.inner_size();

                // let t = start.elapsed().as_secs_f32();

                canvas.set_size(size.width as u32, size.height as u32, dpi_factor as f32);
                canvas.clear_rect(
                    0,
                    0,
                    size.width as u32,
                    size.height as u32,
                    Color::rgbf(0.3, 0.3, 0.32),
                );

                // let height = size.height as f32;
                // let width = size.width as f32;

                let winit::dpi::LogicalSize { width, height: _ } =
                    size.to_logical::<f32>(dpi_factor);

                let pt = canvas
                    .transform()
                    .inversed()
                    .transform_point(mousex, mousey);
                let rel_mousex = pt.0;
                let rel_mousey = pt.1;

                draw_paragraph(
                    &mut canvas,
                    fonts.regular,
                    width - 450.0,
                    50.0,
                    150.0,
                    100.0,
                    rel_mousex,
                    rel_mousey,
                );

                draw_window(
                    &mut canvas,
                    &fonts,
                    "Widgets `n Stuff",
                    50.0,
                    50.0,
                    300.0,
                    400.0,
                );

                let x = 60.0;
                let mut y = 95.0;

                draw_search_box(&mut canvas, &fonts, "Search", x, y, 280.0, 25.0);
                y += 40.0;
                draw_drop_down(&mut canvas, &fonts, "Effects", 60.0, 135.0, 280.0, 28.0);
                y += 45.0;

                draw_label(&mut canvas, &fonts, "Login", x, y, 280.0, 20.0);
                y += 25.0;
                draw_edit_box(&mut canvas, &fonts, "Email", x, y, 280.0, 28.0);
                y += 35.0;
                draw_edit_box(&mut canvas, &fonts, "Password", x, y, 280.0, 28.0);
                y += 38.0;
                draw_check_box(&mut canvas, &fonts, "Remember me", x, y, 140.0, 28.0);
                draw_button(
                    &mut canvas,
                    &fonts,
                    Some("\u{E740}"),
                    "Sign in",
                    x + 138.0,
                    y,
                    140.0,
                    28.0,
                    Color::rgba(0, 96, 128, 255),
                );
                y += 45.0;

                // Slider
                draw_label(&mut canvas, &fonts, "Diameter", x, y, 280.0, 20.0);
                y += 25.0;
                draw_edit_box_num(
                    &mut canvas,
                    &fonts,
                    "123.00",
                    "px",
                    x + 180.0,
                    y,
                    100.0,
                    28.0,
                );
                y += 55.0;

                draw_button(
                    &mut canvas,
                    &fonts,
                    Some("\u{E729}"),
                    "Delete",
                    x,
                    y,
                    160.0,
                    28.0,
                    Color::rgba(128, 16, 8, 255),
                );
                draw_button(
                    &mut canvas,
                    &fonts,
                    None,
                    "Cancel",
                    x + 170.0,
                    y,
                    110.0,
                    28.0,
                    Color::rgba(0, 0, 0, 0),
                );

                /*
                draw_spinner(&mut canvas, 15.0, 285.0, 10.0, t);
                */

                if let Some(image_id) = screenshot_image_id {
                    let x = size.width as f32 - 512.0;
                    let y = size.height as f32 - 512.0;

                    let paint = Paint::image(image_id, x, y, 512.0, 512.0, 0.0, 1.0);

                    let mut path = Path::new();
                    path.rect(x, y, 512.0, 512.0);
                    canvas.fill_path(&mut path, paint);
                    canvas.stroke_path(&mut path, Paint::color(Color::hex("454545")));
                }

                // if true {
                //     let paint = Paint::image(image_id, size.width as f32, 15.0, 1920.0, 1080.0, 0.0, 1.0);
                //     let mut path = Path::new();
                //     path.rect(size.width as f32, 15.0, 1920.0, 1080.0);
                //     canvas.fill_path(&mut path, paint);
                // }

                canvas.save_with(|canvas| {
                    canvas.reset();
                    perf.render(canvas, 5.0, 5.0);
                });

                //canvas.restore();

                canvas.flush();
                #[cfg(not(target_arch = "wasm32"))]
                windowed_context.swap_buffers().unwrap();
            }
            Event::MainEventsCleared => {
                //scroll = 1.0;
                window.request_redraw()
            }
            _ => (),
        }
    });
}

fn draw_paragraph<T: Renderer>(
    canvas: &mut Canvas<T>,
    font: FontId,
    x: f32,
    y: f32,
    width: f32,
    _height: f32,
    mx: f32,
    my: f32,
) {
    let text = "This is longer chunk of text.\n\nWould have used lorem ipsum but she was busy jumping over the lazy dog with the fox and all the men who came to the aid of the party.🎉";

    canvas.save();

    let mut paint = Paint::color(Color::rgba(255, 255, 255, 255));
    paint.set_font_size(14.0);
    paint.set_font(&[font]);
    paint.set_text_align(Align::Left);
    paint.set_text_baseline(Baseline::Top);

    let mut gutter_y = 0.0;
    let mut gutter = 0;
    let mut y = y;
    let mut px;
    let mut caret_x;

    let lines = canvas
        .break_text_vec(width, text, paint)
        .expect("Cannot break text");

    for (line_num, line_range) in lines.into_iter().enumerate() {
        if let Ok(res) = canvas.fill_text(x, y, &text[line_range], paint) {
            let hit = mx > x && mx < (x + width) && my >= y && my < (y + res.height());

            if hit {
                caret_x = if mx < x + res.width() / 2.0 {
                    x
                } else {
                    x + res.width()
                };
                px = x;

                for glyph in &res.glyphs {
                    let x0 = glyph.x;
                    let x1 = x0 + glyph.width;
                    let gx = x0 * 0.3 + x1 * 0.7;

                    if mx >= px && mx < gx {
                        caret_x = glyph.x;
                    }

                    px = gx;
                }

                let mut path = Path::new();
                path.rect(caret_x, y, 1.0, res.height());
                canvas.fill_path(&mut path, Paint::color(Color::rgba(255, 192, 0, 255)));

                gutter = line_num + 1;

                gutter_y = y + 14.0 / 2.0;
            }

            y += res.height();
        }
    }

    if gutter > 0 {
        let mut paint = Paint::color(Color::rgba(255, 192, 0, 255));
        paint.set_font_size(12.0);
        paint.set_font(&[font]);
        paint.set_text_align(Align::Right);
        paint.set_text_baseline(Baseline::Middle);

        let text = format!("{}", gutter);

        if let Ok(res) = canvas.measure_text(x - 10.0, gutter_y, &text, paint) {
            let mut path = Path::new();
            path.rounded_rect(
                res.x - 4.0,
                res.y - 2.0,
                res.width() + 8.0,
                res.height() + 4.0,
                (res.height() + 4.0) / 2.0 - 1.0,
            );
            canvas.fill_path(&mut path, paint);

            paint.set_color(Color::rgba(32, 32, 32, 255));
            let _ = canvas.fill_text(x - 10.0, gutter_y, &text, paint);
        }
    }

    // let mut start = 0;

    // while start < text.len() {
    //     let substr = &text[start..];

    //     if let Ok(index) = canvas.break_text(width, substr, paint) {
    //         if let Ok(res) = canvas.fill_text(x, y, &substr[0..index], paint) {
    //             y += res.height;
    //         }

    //         start += &substr[0..index].len();
    //     } else {
    //         break;
    //     }
    // }

    canvas.restore();
}

fn draw_window<T: Renderer>(
    canvas: &mut Canvas<T>,
    fonts: &Fonts,
    title: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    let corner_radius = 3.0;

    canvas.save();

    //canvas.global_composite_operation(CompositeOperation::Lighter);

    // Window
    let mut path = Path::new();
    path.rounded_rect(x, y, w, h, corner_radius);
    canvas.fill_path(&mut path, Paint::color(Color::rgba(28, 30, 34, 192)));

    // Drop shadow
    let shadow_paint = Paint::box_gradient(
        x,
        y + 2.0,
        w,
        h,
        corner_radius * 2.0,
        10.0,
        Color::rgba(0, 0, 0, 128),
        Color::rgba(0, 0, 0, 0),
    );
    let mut path = Path::new();
    path.rect(x - 10.0, y - 10.0, w + 20.0, h + 30.0);
    path.rounded_rect(x, y, w, h, corner_radius);
    path.solidity(Solidity::Hole);
    canvas.fill_path(&mut path, shadow_paint);

    // Header
    let header_paint = Paint::linear_gradient(
        x,
        y,
        x,
        y + 15.0,
        Color::rgba(255, 255, 255, 8),
        Color::rgba(0, 0, 0, 16),
    );
    let mut path = Path::new();
    path.rounded_rect(x + 1.0, y + 1.0, w - 2.0, 30.0, corner_radius - 1.0);
    canvas.fill_path(&mut path, header_paint);

    let mut path = Path::new();
    path.move_to(x + 0.5, y + 0.5 + 30.0);
    path.line_to(x + 0.5 + w - 1.0, y + 0.5 + 30.0);
    canvas.stroke_path(&mut path, Paint::color(Color::rgba(0, 0, 0, 32)));

    let mut text_paint = Paint::color(Color::rgba(0, 0, 0, 32));
    text_paint.set_font_size(16.0);
    text_paint.set_font(&[fonts.bold]);
    text_paint.set_text_align(Align::Center);
    text_paint.set_color(Color::rgba(220, 220, 220, 160));

    let _ = canvas.fill_text(x + (w / 2.0), y + 19.0, title, text_paint);

    // let bounds = canvas.text_bounds(x + (w / 2.0), y + 19.0, title, text_paint);
    //
    // let mut path = Path::new();
    // path.rect(bounds[0], bounds[1], bounds[2] - bounds[0], bounds[3] - bounds[1]);
    // canvas.stroke_path(&mut path, Paint::color(Color::rgba(0, 0, 0, 255)));

    canvas.restore();
}

fn draw_search_box<T: Renderer>(
    canvas: &mut Canvas<T>,
    fonts: &Fonts,
    title: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    let corner_radius = (h / 2.0) - 1.0;

    let bg = Paint::box_gradient(
        x,
        y + 1.5,
        w,
        h,
        h / 2.0,
        5.0,
        Color::rgba(0, 0, 0, 16),
        Color::rgba(0, 0, 0, 92),
    );
    let mut path = Path::new();
    path.rounded_rect(x, y, w, h, corner_radius);
    canvas.fill_path(&mut path, bg);

    let mut text_paint = Paint::color(Color::rgba(255, 255, 255, 64));
    text_paint.set_font_size((h * 1.3).round());
    text_paint.set_font(&[fonts.icons]);
    text_paint.set_text_align(Align::Center);
    text_paint.set_text_baseline(Baseline::Middle);
    let _ = canvas.fill_text(x + h * 0.55, y + h * 0.55, "\u{1F50D}", text_paint);

    let mut text_paint = Paint::color(Color::rgba(255, 255, 255, 32));
    text_paint.set_font_size(16.0);
    text_paint.set_font(&[fonts.regular]);
    text_paint.set_text_align(Align::Left);
    text_paint.set_text_baseline(Baseline::Middle);
    let _ = canvas.fill_text(x + h, y + h * 0.5, title, text_paint);

    let mut text_paint = Paint::color(Color::rgba(255, 255, 255, 32));
    text_paint.set_font_size((h * 1.3).round());
    text_paint.set_font(&[fonts.icons]);
    text_paint.set_text_align(Align::Center);
    text_paint.set_text_baseline(Baseline::Middle);
    let _ = canvas.fill_text(x + w - h * 0.55, y + h * 0.45, "\u{2716}", text_paint);
}

fn draw_drop_down<T: Renderer>(
    canvas: &mut Canvas<T>,
    fonts: &Fonts,
    title: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    let corner_radius = 4.0;

    let bg = Paint::linear_gradient(
        x,
        y,
        x,
        y + h,
        Color::rgba(255, 255, 255, 16),
        Color::rgba(0, 0, 0, 16),
    );
    let mut path = Path::new();
    path.rounded_rect(x + 1.0, y + 1.0, w - 2.0, h - 2.0, corner_radius);
    canvas.fill_path(&mut path, bg);

    let mut path = Path::new();
    path.rounded_rect(x + 0.5, y + 0.5, w - 1.0, h - 1.0, corner_radius - 0.5);
    canvas.stroke_path(&mut path, Paint::color(Color::rgba(0, 0, 0, 48)));

    let mut text_paint = Paint::color(Color::rgba(255, 255, 255, 160));
    text_paint.set_font_size(16.0);
    text_paint.set_font(&[fonts.regular]);
    text_paint.set_text_align(Align::Left);
    text_paint.set_text_baseline(Baseline::Middle);
    let _ = canvas.fill_text(x + h * 0.3, y + h * 0.5, title, text_paint);

    let mut text_paint = Paint::color(Color::rgba(255, 255, 255, 64));
    text_paint.set_font_size((h * 1.3).round());
    text_paint.set_font(&[fonts.icons]);
    text_paint.set_text_align(Align::Center);
    text_paint.set_text_baseline(Baseline::Middle);
    let _ = canvas.fill_text(x + w - h * 0.5, y + h * 0.45, "\u{E75E}", text_paint);
}

fn draw_label<T: Renderer>(
    canvas: &mut Canvas<T>,
    fonts: &Fonts,
    title: &str,
    x: f32,
    y: f32,
    _w: f32,
    h: f32,
) {
    let mut text_paint = Paint::color(Color::rgba(255, 255, 255, 128));
    text_paint.set_font_size(14.0);
    text_paint.set_font(&[fonts.regular]);
    text_paint.set_text_align(Align::Left);
    text_paint.set_text_baseline(Baseline::Middle);
    let _ = canvas.fill_text(x, y + h * 0.5, title, text_paint);
}

fn draw_edit_box_base<T: Renderer>(canvas: &mut Canvas<T>, x: f32, y: f32, w: f32, h: f32) {
    let paint = Paint::box_gradient(
        x + 1.0,
        y + 2.5,
        w - 2.0,
        h - 2.0,
        3.0,
        4.0,
        Color::rgba(255, 255, 255, 32),
        Color::rgba(32, 32, 32, 32),
    );

    let mut path = Path::new();
    path.rounded_rect(x + 1.0, y + 1.0, w - 2.0, h - 2.0, 3.0);
    canvas.fill_path(&mut path, paint);

    let mut path = Path::new();
    path.rounded_rect(x + 0.5, y + 0.5, w - 1.0, h - 1.0, 3.5);
    canvas.stroke_path(&mut path, Paint::color(Color::rgba(0, 0, 0, 48)));
}

fn draw_edit_box<T: Renderer>(
    canvas: &mut Canvas<T>,
    fonts: &Fonts,
    title: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    draw_edit_box_base(canvas, x, y, w, h);

    let mut text_paint = Paint::color(Color::rgba(255, 255, 255, 64));
    text_paint.set_font_size(16.0);
    text_paint.set_font(&[fonts.regular]);
    text_paint.set_text_align(Align::Left);
    text_paint.set_text_baseline(Baseline::Middle);
    let _ = canvas.fill_text(x + h * 0.5, y + h * 0.5, title, text_paint);
}

fn draw_edit_box_num<T: Renderer>(
    canvas: &mut Canvas<T>,
    fonts: &Fonts,
    title: &str,
    units: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    draw_edit_box_base(canvas, x, y, w, h);

    let mut paint = Paint::color(Color::rgba(255, 255, 255, 64));
    paint.set_font_size(14.0);
    paint.set_font(&[fonts.regular]);
    paint.set_text_align(Align::Right);
    paint.set_text_baseline(Baseline::Middle);

    if let Ok(layout) = canvas.measure_text(0.0, 0.0, units, paint) {
        let _ = canvas.fill_text(x + w - h * 0.3, y + h * 0.5, units, paint);

        paint.set_font_size(16.0);
        paint.set_color(Color::rgba(255, 255, 255, 128));

        let _ = canvas.fill_text(x + w - layout.width() - h * 0.5, y + h * 0.5, title, paint);
    }
}

fn draw_check_box<T: Renderer>(
    canvas: &mut Canvas<T>,
    fonts: &Fonts,
    text: &str,
    x: f32,
    y: f32,
    _w: f32,
    h: f32,
) {
    let mut paint = Paint::color(Color::rgba(255, 255, 255, 160));
    paint.set_font_size(14.0);
    paint.set_font(&[fonts.regular]);
    paint.set_text_baseline(Baseline::Middle);

    let _ = canvas.fill_text(x + 28.0, y + h * 0.5, text, paint);

    paint = Paint::box_gradient(
        x + 1.0,
        y + (h * 0.5).floor() - 9.0 + 1.0,
        18.0,
        18.0,
        3.0,
        3.0,
        Color::rgba(0, 0, 0, 32),
        Color::rgba(0, 0, 0, 92),
    );
    let mut path = Path::new();
    path.rounded_rect(x + 1.0, y + (h * 0.5).floor() - 9.0, 18.0, 18.0, 3.0);
    canvas.fill_path(&mut path, paint);

    paint = Paint::color(Color::rgba(255, 255, 255, 128));
    paint.set_font_size(36.0);
    paint.set_font(&[fonts.icons]);
    paint.set_text_align(Align::Center);
    paint.set_text_baseline(Baseline::Middle);
    let _ = canvas.fill_text(x + 9.0 + 2.0, y + h * 0.5, "\u{2713}", paint);
}

fn draw_button<T: Renderer>(
    canvas: &mut Canvas<T>,
    fonts: &Fonts,
    preicon: Option<&str>,
    text: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
) {
    let corner_radius = 4.0;

    let a = if color.is_black() { 16 } else { 32 };

    let bg = Paint::linear_gradient(
        x,
        y,
        x,
        y + h,
        Color::rgba(255, 255, 255, a),
        Color::rgba(0, 0, 0, a),
    );

    let mut path = Path::new();
    path.rounded_rect(x + 1.0, y + 1.0, w - 2.0, h - 2.0, corner_radius - 1.0);

    if !color.is_black() {
        canvas.fill_path(&mut path, Paint::color(color));
    }

    canvas.fill_path(&mut path, bg);

    let mut path = Path::new();
    path.rounded_rect(x + 0.5, y + 0.5, w - 1.0, h - 1.0, corner_radius - 0.5);
    canvas.stroke_path(&mut path, Paint::color(Color::rgba(0, 0, 0, 48)));

    let mut paint = Paint::color(Color::rgba(255, 255, 255, 96));
    paint.set_font_size(15.0);
    paint.set_font(&[fonts.bold]);
    paint.set_text_align(Align::Left);
    paint.set_text_baseline(Baseline::Middle);

    let tw = if let Ok(layout) = canvas.measure_text(0.0, 0.0, text, paint) {
        layout.width()
    } else {
        0.0
    };

    let mut iw = 0.0;

    if let Some(icon) = preicon {
        paint.set_font(&[fonts.icons]);
        paint.set_font_size(h * 1.3);

        if let Ok(layout) = canvas.measure_text(0.0, 0.0, icon, paint) {
            iw = layout.width() + (h * 0.15);
        }

        let _ = canvas.fill_text(x + w * 0.5 - tw * 0.5 - iw * 0.75, y + h * 0.5, icon, paint);
    }

    paint.set_font_size(15.0);
    paint.set_font(&[fonts.regular]);
    paint.set_color(Color::rgba(0, 0, 0, 160));
    let _ = canvas.fill_text(
        x + w * 0.5 - tw * 0.5 + iw * 0.25,
        y + h * 0.5 - 1.0,
        text,
        paint,
    );
    paint.set_color(Color::rgba(255, 255, 255, 160));
    let _ = canvas.fill_text(x + w * 0.5 - tw * 0.5 + iw * 0.25, y + h * 0.5, text, paint);
}

struct PerfGraph {
    history_count: usize,
    values: Vec<f32>,
    head: usize,
}

impl PerfGraph {
    fn new() -> Self {
        Self {
            history_count: 100,
            values: vec![0.0; 100],
            head: Default::default(),
        }
    }

    fn update(&mut self, frame_time: f32) {
        self.head = (self.head + 1) % self.history_count;
        self.values[self.head] = frame_time;
    }

    fn get_average(&self) -> f32 {
        self.values.iter().map(|v| *v).sum::<f32>() / self.history_count as f32
    }

    fn render<T: Renderer>(&self, canvas: &mut Canvas<T>, x: f32, y: f32) {
        let avg = self.get_average();

        let w = 200.0;
        let h = 35.0;

        let mut path = Path::new();
        path.rect(x, y, w, h);
        canvas.fill_path(&mut path, Paint::color(Color::rgba(0, 0, 0, 128)));

        let mut path = Path::new();
        path.move_to(x, y + h);

        for i in 0..self.history_count {
            let mut v = 1.0 / (0.00001 + self.values[(self.head + i) % self.history_count]);
            if v > 80.0 {
                v = 80.0;
            }
            let vx = x + (i as f32 / (self.history_count - 1) as f32) * w;
            let vy = y + h - ((v / 80.0) * h);
            path.line_to(vx, vy);
        }

        path.line_to(x + w, y + h);
        canvas.fill_path(&mut path, Paint::color(Color::rgba(255, 192, 0, 128)));

        let mut text_paint = Paint::color(Color::rgba(240, 240, 240, 255));
        text_paint.set_font_size(12.0);
        let _ = canvas.fill_text(x + 5.0, y + 13.0, "Frame time", text_paint);

        let mut text_paint = Paint::color(Color::rgba(240, 240, 240, 255));
        text_paint.set_font_size(14.0);
        text_paint.set_text_align(Align::Right);
        text_paint.set_text_baseline(Baseline::Top);
        let _ = canvas.fill_text(x + w - 5.0, y, &format!("{:.2} FPS", 1.0 / avg), text_paint);

        let mut text_paint = Paint::color(Color::rgba(240, 240, 240, 200));
        text_paint.set_font_size(12.0);
        text_paint.set_text_align(Align::Right);
        text_paint.set_text_baseline(Baseline::Alphabetic);
        let _ = canvas.fill_text(
            x + w - 5.0,
            y + h - 5.0,
            &format!("{:.2} ms", avg * 1000.0),
            text_paint,
        );
    }
}
