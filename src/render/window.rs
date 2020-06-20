

use winit::{
    dpi::LogicalSize,
    event::Event,
    event_loop::EventLoop,
    window::Window,
};

use imgui_winit_support::*;

use crate::render::*;

pub mod gui;



pub struct WindowState {
    window: Window,
    platform: WinitPlatform,

    imgui_ctx: imgui::Context,
    
    imgui_renderer: Option<imgui_wgpu::Renderer>,
}


impl WindowState {
    
    pub fn new(event_loop: &EventLoop<()>) -> Self {

        let window = Window::new(&event_loop)
            .expect("Failed to create window.");
        
        window.set_title(&format!("tbs-tcg {}", env!("CARGO_PKG_VERSION")));
        window.set_inner_size(LogicalSize {
            width: 1280, height: 800,
        });
        
        window.set_resizable(true);
        
        let mut hidpi_factor = 1.0f64;
        let mut imgui_ctx = imgui::Context::create();
        
        build_default_font(&mut imgui_ctx, hidpi_factor);

        let mut platform = WinitPlatform::init(&mut imgui_ctx);
        platform.attach_window(
            imgui_ctx.io_mut(),
            &window,
            HiDpiMode::Default,
        );

        Self {
            window,
            platform,
            imgui_ctx,
            imgui_renderer: None,
        }
    }

    pub fn imgui(&mut self) -> &mut imgui::Context {
        &mut self.imgui_ctx
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn platform(&self) -> &WinitPlatform {
        &self.platform
    }
    
    pub fn init_renderer(&mut self, device: &wgpu::Device, queue: &mut wgpu::Queue, format: wgpu::TextureFormat) {
        self.imgui_renderer = Some(imgui_wgpu::Renderer::new(
            &mut self.imgui_ctx, 
            device, 
            queue, 
            format,
            None,
        ));
    }

    pub fn handle_event(&mut self, event: &Event<()>) {
        self.platform.handle_event(self.imgui_ctx.io_mut(), &self.window, event);
    }

    pub fn update_frame_time(&mut self, time: std::time::Instant) -> std::time::Instant {
        self.imgui_ctx.io_mut().update_delta_time(time)
    }

    pub fn update_scale_factor(&mut self, scale_factor: f64) {
        build_default_font(&mut self.imgui_ctx, scale_factor);
    }

    // FIXME: Oh GOD this signature.
    pub fn stage<'r, 's:'r, 'w:'r, 't:'r, 'l:'r, W: gui::Widget>
        (&'s mut self, widget: &'w mut W, lua: &'l mut rlua::Lua, target: &'t wgpu::TextureView)
        -> GuiStage<'r, W> 
    {
        GuiStage {
            window_state: self,
            widget, 
            lua, 
            target,
        }
    }

}


pub struct GuiStage<'r, W: gui::Widget> {
    window_state: &'r mut WindowState,
    widget: &'r mut W,
    target: &'r wgpu::TextureView,
    lua: &'r mut rlua::Lua,
}

impl<W: gui::Widget> crate::render::RenderStage for GuiStage<'_, W> {
    fn encode(self, core: &mut RenderCore, encoder: &mut wgpu::CommandEncoder) {
        
        let WindowState {
            ref mut imgui_ctx,
            ref mut platform,
            ref mut imgui_renderer,
            ref     window,
        } = self.window_state;

        if imgui_renderer.is_none() { return; }
        
        platform.prepare_frame(imgui_ctx.io_mut(), window)
            .expect("Failed to prepare imgui frame.");

        let ui = imgui_ctx.frame();

        self.widget.compose(&ui, &self.lua);

        platform.prepare_render(&ui, window);

        let draw_data = ui.render();
        imgui_renderer.as_mut()
            .unwrap()
            .render(draw_data, core.device, encoder, self.target)
            .expect("Failed to draw imgui.");

    }
}







fn build_default_font(imgui: &mut imgui::Context, hidpi_factor: f64) {
    use imgui::*;

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







