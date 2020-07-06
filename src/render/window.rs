

use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::Window,
};

use imgui_winit_support::*;



pub struct WindowState {
    pub window: Window,
    pub platform: WinitPlatform,

    pub imgui: imgui::Context,
    pub lua: rlua::Lua,
}


impl WindowState {
    
    pub fn new<T>(event_loop: &EventLoop<T>) -> Self {

        let window = Window::new(&event_loop)
            .expect("Failed to create window.");
        
        window.set_title(&format!("tbs-tcg {}", env!("CARGO_PKG_VERSION")));
        window.set_inner_size(LogicalSize {
            width: 1280, height: 800,
        });
        
        window.set_resizable(true);

        let lua = rlua::Lua::new();
        let mut imgui = imgui::Context::create();

        let mut platform = WinitPlatform::init(&mut imgui);
        platform.attach_window(
            imgui.io_mut(),
            &window,
            HiDpiMode::Default,
        );
        
        build_default_font(
            &mut imgui,
            platform.hidpi_factor()
        );

        Self {
            window,
            platform,
            imgui,
            lua,
        }
    }

    pub fn update_frame_time(&mut self, time: std::time::Instant)
        -> (std::time::Instant, std::time::Duration)
    {  
        let io = self.imgui.io_mut();
        let inst = io.update_delta_time(time);
        let dura = std::time::Duration::from_secs_f32(io.delta_time);
        (inst, dura)
    }

    pub fn update_scale_factor(&mut self, scale_factor: f64) {
        build_default_font(&mut self.imgui, scale_factor);
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



