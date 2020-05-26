#![warn(clippy::all)]

use async_std::task;

use imgui::*;
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
    let mut window_state = WindowState::new(&event_loop);

    
    // TODO: convert task::block_on to .await?
    let mut render_state = task::block_on(RenderState::new(&mut window_state));
    
    let lua = &mut window_state.gui.lua;

    lua.context(|ctx: rlua::Context| {
        ctx.load(r#"
            print('hello, world! from ' .. _VERSION)
        "#)
        .exec()
    })
    .expect("Lua printing failed.");
    

    event_loop.run(move |event, _, control_flow| {
        match &event {
            Event::WindowEvent { event, .. } => match *event {
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    window_state.set_scale_factor(scale_factor);
                },
                
                WindowEvent::Resized(new_size) => {
                    window_state.set_size(new_size);
                    render_state.handle_window_resize(&mut window_state);
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
                window_state.as_ref().request_redraw();
            },

            Event::RedrawEventsCleared => {
                render_state.draw_ui(&mut window_state)
                    .expect("Failed to draw ui.");
            },

            _ => (),
        };

        render_state.handle_event(&mut window_state, &event);
    });

}


// #[derive(Debug)]
struct ImguiState {
    last_frametime: time::Instant,
    open_windows: [bool; 1],
    hidpi_factor: f64,
    
    lua: rlua::Lua,
    lua_console: imgui::ImString,
}

impl ImguiState {

    fn new() -> Self {
        Self {
            last_frametime: time::Instant::now(),
            open_windows: [true; 1],
            hidpi_factor: 1.0,
            lua: Lua::new(),
            lua_console: ImString::default(),
        }
    }

    fn inc_frame(&mut self, imgui: &mut imgui::Context) {
        self.last_frametime = 
            imgui.io_mut()
                 .update_delta_time(self.last_frametime);
    }
    

    fn compose_ui<'ui>(&mut self, imgui: &'ui mut imgui::Context) -> imgui::Ui<'ui> {
        let ui = imgui.frame();
        
        ui.show_demo_window(&mut self.open_windows[0]);
     
        let lua_window = imgui::Window::new(im_str!("Lua Console"))
            .size([800.0, 600.0], Condition::FirstUseEver)
            .begin(&ui); 
        
        if let Some(lua_window) = lua_window {
            ui.text(im_str!("should be lua, eventually."));
            
            ui.input_text_multiline(
                im_str!("Console"),
                &mut self.lua_console,
                [600.0, 480.0],
            )
            .allow_tab_input(true)
            .resize_buffer(true)
            .enter_returns_true(false)
            .build();

            if ui.button(im_str!("Run Lua"), [100.0, 50.0]) {
                self.lua.context(|ctx| {
                    match ctx.load(self.lua_console.as_ref() as &str).exec() {
                        Ok(()) => (),
                        Err(e) => eprintln!("Error: {}", e),
                    };
                });
            }

            lua_window.end(&ui);
        }

        ui
    }
}


struct WindowState {
    window: winit::window::Window,
    size: winit::dpi::PhysicalSize<u32>,
    gui: ImguiState,
}

impl AsRef<winit::window::Window> for WindowState {
    fn as_ref(&self) -> &winit::window::Window {
        &self.window
    }
}

impl AsMut<winit::window::Window> for WindowState {
    fn as_mut(&mut self) -> &mut winit::window::Window {
        &mut self.window
    }
}

impl WindowState {

    fn new<T>(event_loop: &EventLoop<T>) -> Self {
        
        let window = Window::new(event_loop).unwrap();
        
        window.set_inner_size(LogicalSize {
            width: 1280, height: 800,
        });
        
        window.set_title(&format!("tbs-tcg {}", env!("CARGO_PKG_VERSION")));

        Self {
            size: window.inner_size(),
            gui: ImguiState::new(),
            window,
        }
    }
    
    fn set_scale_factor(&mut self, hidpi_factor: f64) {
        self.gui.hidpi_factor = hidpi_factor;
    }

    fn set_size(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.size = size;
    }

}



struct RenderState {
    surface: wgpu::Surface,
    // adapter: wgpu::Adapter,
    device:  wgpu::Device,
    queue:   wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    
    renderer: imgui_wgpu::Renderer,
    imgui:    imgui::Context,
    platform: imgui_winit_support::WinitPlatform,
}

impl AsRef<imgui_wgpu::Renderer> for RenderState {
    fn as_ref(&self) -> &imgui_wgpu::Renderer {
        &self.renderer
    }
}

impl AsMut<imgui_wgpu::Renderer> for RenderState {
    fn as_mut(&mut self) -> &mut imgui_wgpu::Renderer {
        &mut self.renderer
    }
}

impl RenderState {

    async fn new(window_state: &mut WindowState) -> Self {

        let surface = wgpu::Surface::create(window_state.as_ref());
        
        let adapter = wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            },
            wgpu::BackendBit::PRIMARY,
        )
        .await
        .expect("Failed to request wgpu::Adapter.");

        let (device, mut queue) = adapter.request_device(&Default::default()).await;

        let sc_desc = wgpu::SwapChainDescriptor {
            usage:  wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm, // WTF: required to some degree by imgui_wgpu ?
            width:  window_state.size.width  as u32,
            height: window_state.size.height as u32,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let mut imgui = imgui::Context::create();

        let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);

        platform.attach_window(
            imgui.io_mut(), 
            window_state.as_ref(), 
            imgui_winit_support::HiDpiMode::Default,
        );

        imgui.io_mut().font_global_scale = (1.0 / window_state.gui.hidpi_factor) as f32;
        imgui.fonts().add_font(&[
            FontSource::DefaultFontData {
                config: Some(FontConfig {
                    oversample_h: 1,
                    pixel_snap_h: true,
                    size_pixels: (13.0 * window_state.gui.hidpi_factor) as f32,
                    ..Default::default()
                })
            }
        ]);

        let clear_color = wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 };
        let renderer = Renderer::new(
            &mut imgui, 
            &device, 
            &mut queue, 
            sc_desc.format, 
            Some(clear_color),
        );

        Self {
            surface,
            // adapter,
            device,
            queue,
            sc_desc,
            swap_chain,

            renderer,
            imgui,
            platform,
        }
    }

    fn draw_ui(&mut self, window_state: &mut WindowState) -> Result<(), Box<dyn std::error::Error>> {

        let Self {
            imgui, 
            device,
            queue, 
            platform, 
            renderer, 
            swap_chain, 
            .. 
        } = self;

        window_state.gui.inc_frame(imgui);
       
        let frame = match swap_chain.get_next_texture() {
            Ok(frame) => frame,
            Err(e) => {
                eprintln!("dropped frame: {:?}", e);
                return Ok(());
            },
        };

        let ui = window_state.gui.compose_ui(imgui);
        platform.prepare_render(&ui, window_state.as_ref());

        let mut encoder = device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("ui") }
        );

        let draw_data = ui.render();
        renderer
            .render(draw_data, device, &mut encoder, &frame.view)
            .map_err(|imgui_wgpu::RendererError::BadTexture(tex_id)| {
                // WTF: because apparently,
                // RendererError is not an std::error::Error...
                format!("Bad texture: {:?}", tex_id)
            })?;

        queue.submit(&[encoder.finish()]);

        Ok(())
    }

    fn handle_event(&mut self, window_state: &mut WindowState, event: &Event<()>) {
        self.platform.handle_event(self.imgui.io_mut(), window_state.as_ref(), event);
    }

    fn handle_window_resize(&mut self, window_state: &mut WindowState) {

        let Self {
            sc_desc,
            swap_chain,
            surface,
            device,
            ..
        } = self;
        
        *sc_desc = wgpu::SwapChainDescriptor {
            width:  window_state.size.width,
            height: window_state.size.height,
            ..*sc_desc
        };

        *swap_chain = device.create_swap_chain(surface, &sc_desc);

    }

}