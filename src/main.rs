use imgui::*;

use imgui_winit_support::{WinitPlatform, HiDpiMode};

use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};


pub(crate) mod gui;
pub(crate) mod render;


use render::RenderCore;


fn build_default_font(imgui: &mut imgui::Context, hidpi_factor: f64) {
    imgui.fonts().clear_fonts();
    imgui.fonts().add_font(&[
        FontSource::DefaultFontData {
            config: Some(FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: (13.0 * hidpi_factor) as f32,
                ..Default::default()
            })
        }
    ]);
}


fn main() -> ! {
    
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
        
    window.set_title(&format!("tbs-tcg {}", env!("CARGO_PKG_VERSION")));
    window.set_inner_size(LogicalSize {
        width: 1280, height: 800,
    });

    window.set_resizable(true);

    
    let mut hidpi_factor = 1.0f64;
    let mut imgui = imgui::Context::create();
    
    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;
    build_default_font(&mut imgui, hidpi_factor);
    
    let mut platform = WinitPlatform::init(&mut imgui);
    platform.attach_window(
        imgui.io_mut(), 
        &window,
        HiDpiMode::Default,
    );

    let mut renderer = futures::executor::block_on(
        RenderCore::init(&window, &mut imgui)
    );
    
    let tri = render::TriangleRenderer::new(&mut renderer);

    let lua = rlua::Lua::new();
    
    // TODO: add ECS processing features
    let mut _world = hecs::World::new();

    let mut widgets = gui::WidgetState::new();

    // Initial Setup
    {
        widgets.add_gui_item(
            gui::ImguiDemoWindow::new()
        );

        widgets.add_gui_item(
            gui::LuaPrintBuffer::new(im_str!("Hello from Lua"))
        );

        widgets.add_gui_item(
            gui::CloneWindow::new(0, [100.0, 100.0])
        );
    }

    let mut last_frame_time = std::time::Instant::now();

    // mainloop
    event_loop.run(move |event, _, control_flow| {

        platform.handle_event(imgui.io_mut(), &window, &event);

        match &event {
            Event::NewEvents(_) => {
                last_frame_time = imgui.io_mut().update_delta_time(last_frame_time);
            },

            Event::WindowEvent { event, .. } => match *event {
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    hidpi_factor = scale_factor;
                    build_default_font(&mut imgui, hidpi_factor);
                },

                WindowEvent::Resized(new_size) => {
                    // WTF: Apparently, this actually causes a panic (!?)
                    // Fortunately it also appears to be unnecessary...
                    // window.set_inner_size(new_size); 
                    renderer.handle_window_resize(new_size);
                    eprintln!("resized: {:?}", new_size);
                },

                WindowEvent::KeyboardInput {
                    input: KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        state: ElementState::Pressed,
                        ..       
                    },
                    ..
                }
                | WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }

                _ => (), //eprintln!("{:?}", event),
            },

            Event::MainEventsCleared => {
                window.request_redraw();
            },
            
            Event::RedrawRequested(_) => {

                let frame = match renderer.swap_chain.get_next_texture() {
                    Ok(frame) => frame,
                    Err(_) => {
                        eprintln!("Dropped frame!");
                        return;
                    }
                };

                renderer.draw(tri.with_target(&frame.view));                

                platform.prepare_frame(imgui.io_mut(), &window)
                    .expect("Failed to prepare frame.");
                
                let ui = imgui.frame();

                widgets.refresh_items();
                let channel = widgets.make_widget_channel();
                for gui_item in widgets.iter_items() {
                    gui_item.compose(&channel, &ui, &lua);
                }
                
                platform.prepare_render(&ui, &window);

                renderer.draw_ui(ui, &frame);
            },

            _ => (),
        };

    });
}

