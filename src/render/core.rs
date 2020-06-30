
use crate::render::window;
use crate::render::{ShaderCache, TextureCache, ModelCache};


pub struct Core {
    pub device: &'static wgpu::Device,
    pub queue: &'static wgpu::Queue,
    surface: wgpu::Surface,
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
    pub shaders: ShaderCache,
    pub textures: TextureCache,
    pub models: ModelCache,
}


impl Core {

    pub async fn init(window_state: &mut window::WindowState) -> Self {

        let instance = wgpu::Instance::new(
            wgpu::BackendBit::PRIMARY,
        );

        let surface = unsafe { instance.create_surface(&window_state.window) };

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            },
            wgpu::UnsafeExtensions::disallow(),
        )
        .await
        .expect("Failed to request wgpu::Adapter.");

        let adapter_info = adapter.get_info();
        println!("{:?}", adapter_info);

        let (device, queue) = adapter.request_device(
            &Default::default(), 
            None, // trace_path
        )
        .await
        .expect("Failed to request wgpu Device/Queue.");
        
        // WTF: We never need to deallocate these during the program,
        // so it's not a big deal if we leak the heap reference. If necessary,
        // a Drop implementation for RenderCore could also handle this.
        let device = Box::leak(Box::new(device));
        let queue  = Box::leak(Box::new(queue));

        let size = window_state.window.inner_size();
        let sc_desc = wgpu::SwapChainDescriptor {
            usage:  wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm, // WTF: for wider compatibility?
            width:  size.width  as u32,
            height: size.height as u32,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let shaders = ShaderCache::new(device);
        let textures = TextureCache::new(device, queue);
        let models = ModelCache::new(device);

        Self {
            device,
            queue,
            surface,
            sc_desc,
            swap_chain,
            shaders,
            textures,
            models,
        }
    }

    pub fn handle_window_resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {

        self.sc_desc = wgpu::SwapChainDescriptor {
            width:  size.width,
            height: size.height,
            ..self.sc_desc
        };

        self.swap_chain =
            self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

}
