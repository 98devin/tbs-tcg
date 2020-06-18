
use winit::window::Window;

pub(crate) mod shade;



pub trait Encodable {
    fn encode(&mut self, encoder: &mut wgpu::CommandEncoder);
}


pub struct RenderCore {
    
    pub device: &'static wgpu::Device,
    pub queue:  wgpu::Queue,
    
    surface: wgpu::Surface,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,

    pub shaders: shade::ShaderCache,

    // TODO: Replace this with in-crate code, or
    // separate rendering passes into external objects.
    imgui_renderer: imgui_wgpu::Renderer,

}


impl RenderCore {

    pub async fn init(window: &Window, imgui: &mut imgui::Context) -> Self {

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
        
        // WTF: We never need to deallocate this device during the program,
        // so it's not a big deal if we leak the heap reference. If necessary,
        // a Drop implementation for RenderCore could also handle this.
        let device = Box::leak(Box::new(device));

        let size = window.inner_size();
        let sc_desc = wgpu::SwapChainDescriptor {
            usage:  wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm, // WTF: required to some degree ?
            width:  size.width  as u32,
            height: size.height as u32,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let clear_color = wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 0.2 };
        let imgui_renderer = imgui_wgpu::Renderer::new(
            imgui, 
            &device, 
            &mut queue, 
            sc_desc.format, 
            Some(clear_color),
        );

        let shaders = shade::ShaderCache::new(device);

        RenderCore {
            surface,
            device,
            queue,
            sc_desc,
            shaders,
            swap_chain,
            imgui_renderer,
        }
    }

    pub fn draw_ui(&mut self, ui: imgui::Ui) {

        let Self {
            device,
            queue,
            imgui_renderer, 
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
        imgui_renderer
            .render(draw_data, device, &mut encoder, &frame.view)
            .expect("Failed to draw ui.");

        queue.submit(&[encoder.finish()]);
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


pub struct TriangleRenderer {
    layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
}


impl TriangleRenderer {
    pub fn new(core: &mut RenderCore) -> Self {
        use wgpu::*;

        let layout_descriptor = wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[]
        };

        let layout = core.device.create_pipeline_layout(&layout_descriptor);
        

        let vert_module = core.shaders.load("trivial.vert");
        let frag_module = core.shaders.load("trivial.frag");

        let render_descriptor = RenderPipelineDescriptor {
            layout: &layout,
            
            vertex_stage: vert_module.descriptor(),
            fragment_stage: Some(frag_module.descriptor()),
            
            rasterization_state: Some(RasterizationStateDescriptor {
                front_face: FrontFace::Ccw,
                cull_mode: CullMode::None, // nothing to cull
                ..Default::default()
            }),
            
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            
            color_states: &[wgpu::ColorStateDescriptor {
                format: core.sc_desc.format,
                color_blend: BlendDescriptor {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha_blend: BlendDescriptor {
                    src_factor: BlendFactor::OneMinusDstAlpha,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                write_mask: ColorWrite::ALL,
            }],
            
            depth_stencil_state: None,
            
            vertex_state: VertexStateDescriptor {
                index_format: IndexFormat::Uint16,
                vertex_buffers: &[],
            },
            
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let pipeline = core.device.create_render_pipeline(&render_descriptor);

        Self {
            layout,
            pipeline,
        }
    }
}
