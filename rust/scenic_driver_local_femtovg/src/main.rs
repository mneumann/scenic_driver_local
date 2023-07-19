use std::num::NonZeroU32;

use femtovg::{
    renderer::OpenGl, Align, Baseline, Canvas, Color, FillRule, FontId, ImageFlags, ImageId,
    LineCap, LineJoin, Paint, Path, Renderer, Solidity,
};

use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder},
    display::GetGlDisplay,
    prelude::*,
    surface::{SurfaceAttributesBuilder, WindowSurface},
};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasRawWindowHandle;
use winit::{
    event::{ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

fn main() {
    start(640, 480, "Hello", false);
}
struct Fonts {
    regular: FontId,
    bold: FontId,
    icons: FontId,
}

fn draw_label<T: Renderer>(
    canvas: &mut Canvas<T>,
    font: FontId,
    title: &str,
    x: f32,
    y: f32,
    _w: f32,
    h: f32,
    align: Align,
) {
    let mut text_paint = Paint::color(Color::rgba(255, 255, 255, 128));
    text_paint.set_font_size(14.0);
    text_paint.set_font(&[font]);
    text_paint.set_text_align(align);
    text_paint.set_text_baseline(Baseline::Middle);
    let _ = canvas.fill_text(x, y + h * 0.5, title, &text_paint);
}

fn start(width: u32, height: u32, title: &'static str, resizeable: bool) {
    let event_loop = EventLoop::new();

    let (canvas, window, context, surface) = {
        let window_builder = WindowBuilder::new()
            .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
            .with_resizable(resizeable)
            .with_title(title);

        let template = ConfigTemplateBuilder::new().with_alpha_size(8);

        let display_builder = DisplayBuilder::new().with_window_builder(Some(window_builder));

        let (window, gl_config) = display_builder
            .build(&event_loop, template, |configs| {
                // Find the config with the maximum number of samples, so our triangle will
                // be smooth.
                configs
                    .reduce(|accum, config| {
                        let transparency_check = config.supports_transparency().unwrap_or(false)
                            & !accum.supports_transparency().unwrap_or(false);

                        if transparency_check || config.num_samples() < accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .unwrap()
            })
            .unwrap();

        let window = window.unwrap();

        let raw_window_handle = Some(window.raw_window_handle());

        let gl_display = gl_config.display();

        let context_attributes = ContextAttributesBuilder::new().build(raw_window_handle);
        let fallback_context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(None))
            .build(raw_window_handle);
        let mut not_current_gl_context = Some(unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .unwrap_or_else(|_| {
                    gl_display
                        .create_context(&gl_config, &fallback_context_attributes)
                        .expect("failed to create context")
                })
        });

        let (width, height): (u32, u32) = window.inner_size().into();
        let raw_window_handle = window.raw_window_handle();
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            raw_window_handle,
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
        );

        let surface = unsafe {
            gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
                .unwrap()
        };

        let gl_context = not_current_gl_context
            .take()
            .unwrap()
            .make_current(&surface)
            .unwrap();

        let renderer = unsafe {
            OpenGl::new_from_function_cstr(|s| gl_display.get_proc_address(s) as *const _)
        }
        .expect("Cannot create renderer");

        let mut canvas = Canvas::new(renderer).expect("Cannot create canvas");
        canvas.set_size(width, height, window.scale_factor() as f32);

        (canvas, window, gl_context, surface)
    };

    run(canvas, event_loop, context, surface, window);
}

fn run(
    mut canvas: Canvas<OpenGl>,
    el: EventLoop<()>,
    context: glutin::context::PossiblyCurrentContext,
    surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
    window: Window,
) {
    let font_normal : FontId = canvas.add_font("assets/Roboto-Regular.ttf").unwrap();
    let font_arabic : FontId = canvas.add_font("assets/NotoSansArabic-Black.ttf").unwrap();


    let mut mousex = 0.0;
    let mut mousey = 0.0;
    let mut dragging = false;

    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::LoopDestroyed => *control_flow = ControlFlow::Exit,
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(physical_size) => {
                    surface.resize(
                        &context,
                        physical_size.width.try_into().unwrap(),
                        physical_size.height.try_into().unwrap(),
                    );
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
                    delta: winit::event::MouseScrollDelta::LineDelta(_, y),
                    ..
                } => {
                    let pt = canvas
                        .transform()
                        .inversed()
                        .transform_point(mousex, mousey);
                    canvas.translate(pt.0, pt.1);
                    canvas.scale(1.0 + (y / 10.0), 1.0 + (y / 10.0));
                    canvas.translate(-pt.0, -pt.1);
                }
                WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    state,
                    ..
                } => match state {
                    ElementState::Pressed => dragging = true,
                    ElementState::Released => dragging = false,
                },
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _ => (),
            },
            Event::RedrawRequested(_) => {
                let dpi_factor = window.scale_factor();
                let size = window.inner_size();

                canvas.set_size(size.width, size.height, dpi_factor as f32);
                canvas.clear_rect(0, 0, size.width, size.height, Color::rgbf(0.3, 0.3, 0.32));

                draw_label(
                    &mut canvas,
                    font_normal,
                    "Hallo Leute",
                    200.0,
                    100.0,
                    280.0,
                    20.0,
                    Align::Left,
                );

                draw_label(
                    &mut canvas,
                    font_arabic,
                    "سلام دوستان",
                    200.0,
                    150.0,
                    280.0,
                    20.0,
                    Align::Right,
                );

                canvas.flush();
                surface.swap_buffers(&context).unwrap();
            }
            Event::MainEventsCleared => window.request_redraw(),
            _ => (),
        }
    });
}
