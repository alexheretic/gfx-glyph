extern crate env_logger;
extern crate gfx;
extern crate gfx_glyph;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate spin_sleep;

use gfx::{format, Device};
use gfx_glyph::*;
use glutin::GlContext;
use std::env;

fn main() {
    env_logger::init();

    if cfg!(target_os = "linux") {
        // winit wayland is currently still wip
        if env::var("WINIT_UNIX_BACKEND").is_err() {
            env::set_var("WINIT_UNIX_BACKEND", "x11");
        }
        // disables vsync sometimes on x11
        if env::var("vblank_mode").is_err() {
            env::set_var("vblank_mode", "0");
        }
    }
    if cfg!(debug_assertions) && env::var("yes_i_really_want_debug_mode").is_err() {
        eprintln!("You should probably run an example called 'performance' in release mode, \
            don't you think?\n    \
            e.g. use `cargo run --example performance --release`\n\n\
            If you really want to see debug performance set env var `yes_i_really_want_debug_mode`");
        return;
    }

    let mut events_loop = glutin::EventsLoop::new();
    let title = "gfx_glyph rendering 100,000 glyphs - scroll to size, type to modify";
    let window_builder = glutin::WindowBuilder::new()
        .with_title(title)
        .with_dimensions(1024, 576);
    let context = glutin::ContextBuilder::new().with_vsync(false);
    let (window, mut device, mut factory, mut main_view, mut main_depth) =
        gfx_window_glutin::init::<format::Srgba8, format::Depth>(
            window_builder,
            context,
            &events_loop,
        );

    let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(include_bytes!("DejaVuSans.ttf") as &[u8])
        .initial_cache_size((2048, 2048))
        .gpu_cache_position_tolerance(1.0)
        .build(factory.clone());

    let mut text: String = include_str!("loads-of-unicode.txt").into();
    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut running = true;
    let mut font_size = Scale::uniform(25.0 * window.hidpi_factor());
    let mut loop_helper = spin_sleep::LoopHelper::builder().build_without_target_rate();

    while running {
        loop_helper.loop_start();

        events_loop.poll_events(|event| {
            use glutin::*;

            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::Closed => running = false,
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(keypress),
                                ..
                            },
                        ..
                    } => match keypress {
                        VirtualKeyCode::Escape => running = false,
                        VirtualKeyCode::Back => {
                            text.pop();
                        }
                        _ => (),
                    },
                    WindowEvent::ReceivedCharacter(c) => if c != '\u{7f}' && c != '\u{8}' {
                        text.push(c);
                    },
                    WindowEvent::Resized(width, height) => {
                        window.resize(width, height);
                        gfx_window_glutin::update_views(&window, &mut main_view, &mut main_depth);
                    }
                    WindowEvent::MouseWheel {
                        delta: MouseScrollDelta::LineDelta(_, y),
                        ..
                    } => {
                        // increase/decrease font size with mouse wheel
                        let mut size = font_size.x / window.hidpi_factor();
                        if y > 0.0 {
                            size += (size / 4.0).max(2.0)
                        }
                        else {
                            size *= 4.0 / 5.0
                        };
                        size = size.max(1.0);
                        font_size = Scale::uniform(size * window.hidpi_factor());
                    }
                    _ => {}
                }
            }
        });

        encoder.clear(&main_view, [0.02, 0.02, 0.02, 1.0]);

        let (width, height, ..) = main_view.get_dimensions();
        let (width, height) = (f32::from(width), f32::from(height));

        // The section is all the info needed for the glyph brush to render a 'section' of text
        // can use `..Section::default()` to skip the bits you don't care about
        let section = Section {
            text: &text,
            scale: font_size,
            bounds: (width, height),
            color: [0.8, 0.8, 0.8, 1.0],
            layout: Layout::default().line_breaker(BuiltInLineBreaker::AnyCharLineBreaker),
            ..Section::default()
        };

        // Adds a section & layout to the queue for the next call to `draw_queued`, this
        // can be called multiple times for different sections that want to use the same
        // font and gpu cache
        // This step computes the glyph positions, this is cached to avoid unnecessary recalculation
        glyph_brush.queue(&section);

        // Finally once per frame you want to actually draw all the sections you've submitted
        // with `queue` calls.
        //
        // Note: Drawing in the case the text is unchanged from the previous frame (a common case)
        // is essentially free as the vertices are reused &  gpu cache updating interaction
        // can be skipped.
        glyph_brush
            .draw_queued(&mut encoder, &main_view, &main_depth)
            .expect("draw");

        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();

        if let Some(rate) = loop_helper.report_rate() {
            window.set_title(&format!("{} - {:.0} FPS", title, rate));
        }
    }
    println!();
}
