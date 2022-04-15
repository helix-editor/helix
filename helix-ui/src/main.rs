use parley::{
    layout::Alignment,
    style::{FontFamily, FontStack, StyleProperty},
    FontContext, LayoutContext,
};
use std::borrow::Cow;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use wgpu::util::DeviceExt;

// new femto-like framework:
// wgpu renderer
// kurbo, (alternative is euclid + lyon)
// vector math: glam? and drop euclid (glam is faster https://docs.rs/glam/latest/glam/)
// swash + parley for text

// imgref, bitflags
// fnv, rgb

// resource, image
// usvg for svg

use swash::{
    scale::ScaleContext,
    shape::ShapeContext,
    text::Script,
    zeno::{Vector, Verb},
    Attributes, CacheKey, Charmap, FontRef,
};

use lyon::{
    math::{point, Transform},
    path::{builder::*, Path},
    tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers},
};

use bytemuck::{Pod, Zeroable};

// Vertex for lines drawn by lyon
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    // color: [f32; 4],    // Use this when I want more colors
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct View {
    size: [f32; 2],
}

pub struct Font {
    // Full content of the font file
    data: Vec<u8>,
    // Offset to the table directory
    offset: u32,
    // Cache key
    key: CacheKey,
}

impl Font {
    pub fn from_file(path: &str, index: usize) -> Option<Self> {
        // Read the full font file
        let data = std::fs::read(path).ok()?;
        // Create a temporary font reference for the first font in the file.
        // This will do some basic validation, compute the necessary offset
        // and generate a fresh cache key for us.
        let font = FontRef::from_index(&data, index)?;
        let (offset, key) = (font.offset, font.key);
        // Return our struct with the original file data and copies of the
        // offset and key from the font reference
        Some(Self { data, offset, key })
    }

    // As a convenience, you may want to forward some methods.
    pub fn attributes(&self) -> Attributes {
        self.as_ref().attributes()
    }

    pub fn charmap(&self) -> Charmap {
        self.as_ref().charmap()
    }

    // Create the transient font reference for accessing this crate's
    // functionality.
    pub fn as_ref(&self) -> FontRef {
        // Note that you'll want to initialize the struct directly here as
        // using any of the FontRef constructors will generate a new key which,
        // while completely safe, will nullify the performance optimizations of
        // the caching mechanisms used in this crate.
        FontRef {
            data: &self.data,
            offset: self.offset,
            key: self.key,
        }
    }
}

fn font() -> VertexBuffers<Vertex, u16> {
    // let font = Font::from_file("assets/fonts/Inter Variable/Inter.ttf", 0).unwrap();
    let font = Font::from_file("assets/fonts/ttf/FiraCode-Regular.ttf", 0).unwrap();
    let font = font.as_ref();

    // -- Shaping

    let mut context = ShapeContext::new();
    let mut shaper = context
        .builder(font)
        .script(Script::Latin)
        .size(12.)
        .variations(&[("wght", 400.0)])
        .build();

    shaper.add_str("a quick brown fox?");

    // add_str with boundary analysis
    // use swash::text::{analyze, Script};
    // use swash::text::cluster::{CharInfo, Parser, Token};
    // let text = "a quick brown fox?";
    // let mut parser = Parser::new(
    //     Script::Latin,
    //     text.char_indices()
    //         // Call analyze passing the same text and zip
    //         // the results
    //         .zip(analyze(text.chars()))
    //         // Analyze yields the tuple (Properties, Boundary)
    //         .map(|((i, ch), (props, boundary))| Token {
    //             ch,
    //             offset: i as u32,
    //             len: ch.len_utf8() as u8,
    //             // Create character information from properties and boundary
    //             info: CharInfo::new(props, boundary),
    //             data: 0,
    //         }),
    // );

    shaper.shape_with(|c| {
        // use the glyph cluster

        // c.glyphs
    });

    // -- Scaling

    let mut context = ScaleContext::new();
    let mut scaler = context
        .builder(font)
        .hint(true)
        .size(12.)
        .variations(&[("wght", 400.0)])
        .build();
    let glyph_id = font.charmap().map('H');
    let outline = scaler.scale_outline(glyph_id).unwrap();

    // -- Layout

    let mut font_ctx = FontContext::new();
    let font_family = font_ctx.register_fonts(font.data.to_vec()).unwrap();
    let mut layout_ctx: LayoutContext<[u8; 4]> = LayoutContext::new();

    // Encode glyphs into lyon paths
    let mut encoder = Path::builder();
    let mut encoder = encoder.transformed(Transform::default());

    let mut builder = layout_ctx.ranged_builder(&mut font_ctx, "fn draw_edit_box_base<T: Renderer>(canvas: &mut Canvas<T>, x: f32, y: f32, w: f32, h: f32) { ", 1.);
    builder.push_default(&StyleProperty::FontStack(FontStack::Single(
        FontFamily::Named(&font_family),
    )));
    builder.push_default(&StyleProperty::FontSize(12.));
    builder.push_default(&StyleProperty::Brush([255, 255, 255, 255]));
    // builder.push() with range to set styling
    let mut layout = builder.build();
    let max_width = None;
    layout.break_all_lines(max_width, Alignment::Start);

    for line in layout.lines() {
        let mut last_x = 0.0;
        let mut last_y = 0.0;

        for glyph_run in line.glyph_runs() {
            let run = glyph_run.run();
            // let color = &glyph_run.style().brush.0;
            let font = run.font();
            let font = font.as_ref();

            let mut first = true;

            // TODO: move let scaler here
            for glyph in glyph_run.positioned_glyphs() {
                let delta_x = glyph.x - last_x;
                let delta_y = glyph.y - last_y;

                last_x = glyph.x;
                last_y = glyph.y;

                if first {
                    // TODO:
                }
                first = false;

                // TODO: each glyph will need a translate+scale along with the glyph
                // or we could run the pipeline per letter?

                encoder.set_transform(Transform::new(
                    1.0, 0.0, //
                    0.0, -1.0, // invert y axis
                    glyph.x, glyph.y,
                ));

                if let Some(outline) = scaler.scale_outline(glyph.id) {
                    append_outline(&mut encoder, outline.verbs(), outline.points());
                };
            }
        }
    }

    // -- Tesselation
    let path = encoder.build();

    let mut geometry: VertexBuffers<Vertex, u16> = VertexBuffers::new();

    let mut tessellator = FillTessellator::new();
    {
        // Compute the tessellation.
        tessellator
            .tessellate_path(
                &path,
                &FillOptions::non_zero().with_tolerance(0.01), // defaults to 0.1, compare further
                &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| Vertex {
                    position: vertex.position().to_array(),
                }),
            )
            .unwrap();
    }

    geometry
}

fn append_outline<T: lyon::path::builder::PathBuilder>(
    encoder: &mut T,
    verbs: &[Verb],
    points: &[Vector],
) {
    let mut i = 0;
    for verb in verbs {
        match verb {
            Verb::MoveTo => {
                let p = points[i];
                // TODO: can MoveTo appear halfway through?
                encoder.begin(point(p.x, p.y));
                i += 1;
            }
            Verb::LineTo => {
                let p = points[i];
                encoder.line_to(point(p.x, p.y));
                i += 1;
            }
            Verb::QuadTo => {
                let p1 = points[i];
                let p2 = points[i + 1];
                encoder.quadratic_bezier_to(point(p1.x, p1.y), point(p2.x, p2.y));
                i += 2;
            }
            Verb::CurveTo => {
                let p1 = points[i];
                let p2 = points[i + 1];
                let p3 = points[i + 2];
                encoder.cubic_bezier_to(point(p1.x, p1.y), point(p2.x, p2.y), point(p3.x, p3.y));
                i += 3;
            }
            Verb::Close => {
                encoder.close();
            }
        }
    }
}

fn create_multisampled_framebuffer(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    sample_count: u32,
) -> wgpu::TextureView {
    let multisampled_texture_extent = wgpu::Extent3d {
        width: config.width,
        height: config.height,
        depth_or_array_layers: 1,
    };
    let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
        size: multisampled_texture_extent,
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
    };

    device
        .create_texture(multisampled_frame_descriptor)
        .create_view(&wgpu::TextureViewDescriptor::default())
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let sample_count = 4;

    let size = window.inner_size();
    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let surface = unsafe { instance.create_surface(&window) };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(), // TODO: select based on backend
            force_fallback_adapter: false,
            // Request an adapter which can render to our surface
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    // Create the logical device and command queue
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
            },
            None,
        )
        .await
        .expect("Failed to create device");

    // Load the shaders from disk
    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
    });

    // ---

    let geometry = font();

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&geometry.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(&geometry.indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    //

    // TODO: use size fetched before
    let data = View { size: [0.0, 0.0] };

    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::cast_slice(&[data]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let uniform_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("uniform_bind_group_layout"),
        });

    let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &uniform_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
        label: Some("uniform_bind_group"),
    });

    //

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&uniform_bind_group_layout], // &texture_bind_group_layout
        push_constant_ranges: &[], // TODO: could use push constants for uniforms but that's not available on web
    });

    let swapchain_format = surface.get_preferred_format(&adapter).unwrap();

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[swapchain_format.into()],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        multiview: None,
    });

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    let mut multisampled_framebuffer =
        create_multisampled_framebuffer(&device, &config, sample_count);

    surface.configure(&device, &config);

    //

    event_loop.run(move |event, _, control_flow| {
        // Have the closure take ownership of the resources.
        // `event_loop.run` never returns, therefore we must do this to ensure
        // the resources are properly cleaned up.
        let _ = (&instance, &adapter, &shader, &pipeline_layout);

        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                // Reconfigure the surface with the new size
                config.width = size.width;
                config.height = size.height;

                multisampled_framebuffer =
                    create_multisampled_framebuffer(&device, &config, sample_count);

                surface.configure(&device, &config);
                // On macos the window needs to be redrawn manually after resizing
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let frame = surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                // TODO: need to use queue.write_buffer or staging_belt to write to it

                // Pass the current window size in
                let dpi_factor = window.scale_factor();
                let size = window.inner_size();
                let winit::dpi::LogicalSize { width, height } = size.to_logical::<f32>(dpi_factor);

                let data = View {
                    size: [width, height],
                };

                queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[data]));

                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[wgpu::RenderPassColorAttachment {
                            view: &multisampled_framebuffer,
                            resolve_target: Some(&view),
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });

                    // rpass.set_viewport();

                    rpass.set_pipeline(&render_pipeline);
                    rpass.set_bind_group(0, &uniform_bind_group, &[]);
                    rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    rpass.draw_indexed(0..(geometry.indices.len() as u32), 0, 0..1);
                }

                queue.submit(Some(encoder.finish()));
                frame.present();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}

fn main() {
    let event_loop = EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        // Temporarily avoid srgb formats for the swapchain on the web
        pollster::block_on(run(event_loop, window));
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        use winit::platform::web::WindowExtWebSys;
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
        wasm_bindgen_futures::spawn_local(run(event_loop, window));
    }
}
