
use async_std::task;

use imgui::*;
use imgui_winit_support;
use imgui_wgpu::Renderer;


use rlua::{Lua};

use std::time;

use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};


fn main() { // -> Result<(), Box<dyn std::error::Error>> {

    let event_loop = EventLoop::new();
    let mut hidpi_factor = 1.0;
    let (window, mut size, surface) = {
        let version = env!("CARGO_PKG_VERSION");

        let window = Window::new(&event_loop).unwrap();
        window.set_inner_size(LogicalSize {
            width: 800, height: 600,
        });
        window.set_title(&format!("tbs-tcg {}", version));
        
        let size = window.inner_size();
        let surface = wgpu::Surface::create(&window);
        
        (window, size, surface)
    };

    // TODO: Eventually move this to some thread executor used application-wide
    let adapter = task::block_on(wgpu::Adapter::request(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::Default,
            compatible_surface: Some(&surface),
        },
        wgpu::BackendBit::PRIMARY,
    ))
    .expect("Failed to acquire graphics adapter.");

    let (mut device, mut queue) = task::block_on(
        adapter.request_device(&wgpu::DeviceDescriptor::default())
    );

    let mut sc_desc = wgpu::SwapChainDescriptor {
        usage:  wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8Unorm,
        width:  size.width  as u32,
        height: size.height as u32,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);

    let mut imgui = imgui::Context::create();
    let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    platform.attach_window(imgui.io_mut(), &window, imgui_winit_support::HiDpiMode::Default);

    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;
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

    let clear_color = wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 };
    let mut renderer = Renderer::new(
        &mut imgui, &device, &mut queue, sc_desc.format, Some(clear_color)
    );

    let mut gui = ImguiState { 
        last_cursor: None,
        last_frame: time::Instant::now(),
        open_windows: [true; 3],
    };

    let lua = Lua::new();
    lua.context(|ctx: rlua::Context| {
        let hw = ctx.load("print('hello, world!')");
        hw.exec()
    })
    .expect("Lua printing failed.");
    

    event_loop.run(move |event, _, control_flow| {
        match &event {
            Event::WindowEvent { event, .. } => match *event {
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    hidpi_factor = scale_factor;
                },
                
                WindowEvent::Resized(_) => {
                    size = window.inner_size();

                    sc_desc = wgpu::SwapChainDescriptor {
                        width:  size.width,
                        height: size.height,
                        ..sc_desc
                    };

                    swap_chain = device.create_swap_chain(&surface, &sc_desc);
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

                _ => (),
            },

            Event::MainEventsCleared => {
                window.request_redraw();
            },

            Event::RedrawEventsCleared => {

                gui.inc_frame(&mut imgui);

                let frame = match swap_chain.get_next_texture() {
                    Ok(frame) => frame,
                    Err(e) => {
                        eprintln!("dropped frame: {:?}", e);
                        return;
                    },
                };

                platform
                    .prepare_frame(imgui.io_mut(), &window)
                    .expect("Failed to prepare frame.");


                

                let ui = imgui.frame();

                gui.compose_gui(&ui);
                gui.update_cursor(&ui);
                
                platform.prepare_render(&ui, &window);
                
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                renderer
                    .render(ui.render(), &mut device, &mut encoder, &frame.view)
                    .expect("Failed to render.");
                
                queue.submit(&[encoder.finish()]);
            },

            _ => (),
        };

        platform.handle_event(imgui.io_mut(), &window, &event);
    });

}


#[derive(Debug)]
struct ImguiState {
    imgui: imgui::Context,
    last_frametime: time::Instant,
    open_windows: [bool; 3],
    last_cursor: Option<MouseCursor>,
}

impl ImguiState {

    fn inc_frame(&mut self, imgui: &mut imgui::Context) {
        self.last_frametime = 
            imgui.io_mut()
                 .update_delta_time(self.last_frametime);
    }

    fn compose_gui(&mut self, ui: &imgui::Ui) {
        ui.show_about_window(&mut self.open_windows[0]);
        ui.show_demo_window(&mut self.open_windows[1]);
        ui.show_metrics_window(&mut self.open_windows[2]);
    }

    fn update_cursor(&mut self, ui: &imgui::Ui) -> bool {
        let next_cursor = ui.mouse_cursor();
        if self.last_cursor != next_cursor {
            self.last_cursor = next_cursor;
            return true
        } else {
            return false
        }
    }
}


struct WindowState {
    window: winit::window::Window,
    size: winit::dpi::PhysicalSize<u32>,
    
    renderer: imgui_wgpu::Renderer,

    platform: imgui_winit_support::WinitPlatform,
    gui_state: ImguiState,
}

struct RenderState {
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device:  wgpu::Device,
    queue:   wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,

    window_state: WindowState,
}
