#![warn(clippy::all)]

use imgui::*;
use imgui_wgpu::Renderer;

use imgui_winit_support::{WinitPlatform, HiDpiMode};

use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

use legion::prelude::*;

mod gui;

type LuaAccess = parking_lot::Mutex<rlua::Lua>;


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

    let mut render_state = futures::executor::block_on(
        RenderState::init(&window, &mut imgui)
    );
    
    let lua = LuaAccess::new(rlua::Lua::new());
    
    let universe = Universe::new();
    let mut world = universe.create_world();

    let mut widgets = gui::ImguiWidgets::new();

    // Initial Setup
    {
        world.resources.insert(lua);

        widgets.add_widget(
            gui::ImguiDemoWindow::new()
        );

        widgets.add_widget(
            gui::LuaPrintBuffer::new(im_str!("Hello from Lua"))
        );
    }

    let mut last_frame_time = std::time::Instant::now();
    
    // mainloop
    event_loop.run(move |event, _, control_flow| {

        platform.handle_event(imgui.io_mut(), &window, &event);

        match &event {
            Event::WindowEvent { event, .. } => match *event {
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    hidpi_factor = scale_factor;
                    build_default_font(&mut imgui, hidpi_factor);
                },
                
                WindowEvent::Resized(new_size) => {
                    window.set_inner_size(new_size);
                    render_state.handle_window_resize(new_size);
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

            Event::RedrawRequested(_) => {

                last_frame_time = imgui.io_mut().update_delta_time(last_frame_time);
                platform.prepare_frame(imgui.io_mut(), &window)
                    .expect("Failed to prepare frame.");

                let ui = imgui.frame();
                
                let lua = world.resources.get::<LuaAccess>().unwrap();
                let lua = lua.lock();

                let items = widgets.iter_items(); // takes ownership
                for gui_item in items {
                    gui_item.compose(&mut widgets, &ui, &lua);
                }

                drop(lua); // release mutex
                
                platform.prepare_render(&ui, &window);

                render_state.draw_ui(ui);
            },

            _ => (),
        };
    });
}





struct RenderState {
    surface: wgpu::Surface,
    // adapter: wgpu::Adapter,
    device:  wgpu::Device,
    queue:   wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    
    renderer: imgui_wgpu::Renderer,
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

    async fn init(window: &Window, imgui: &mut imgui::Context) -> Self {

        let surface = wgpu::Surface::create(window);
        
        let adapter = wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            },
            wgpu::BackendBit::PRIMARY,
        )
        .await
        .expect("Failed to request wgpu::Adapter.");

        let adapter_info = adapter.get_info();
        println!("{:?}", adapter_info);

        let (device, mut queue) = adapter.request_device(&Default::default()).await;

        let size = window.inner_size();
        let sc_desc = wgpu::SwapChainDescriptor {
            usage:  wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm, // WTF: required to some degree by imgui_wgpu ?
            width:  size.width  as u32,
            height: size.height as u32,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let clear_color = wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 };
        let renderer = Renderer::new(
            imgui, 
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
        }
    }

    fn draw_ui(&mut self, ui: imgui::Ui) {

        let Self {
            device,
            queue,
            renderer, 
            swap_chain, 
            .. 
        } = self;

        let frame = match swap_chain.get_next_texture() {
            Ok(frame) => frame,
            Err(e) => {
                eprintln!("dropped frame: {:?}", e);
                return;
            },
        };

        let mut encoder = device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("imgui") }
        );

        let draw_data = ui.render();
        renderer
            .render(draw_data, device, &mut encoder, &frame.view)
            .expect("Failed to draw ui.");

        queue.submit(&[encoder.finish()]);
    }

    fn handle_window_resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {

        self.sc_desc = wgpu::SwapChainDescriptor {
            width:  size.width,
            height: size.height,
            ..self.sc_desc
        };

        self.swap_chain =
            self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

}