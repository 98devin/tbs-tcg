
use winit::window::Window;

pub(crate) mod shade;
pub(crate) mod window;


pub trait Vertex {
    fn buffer_descriptor() -> wgpu::VertexBufferDescriptor<'static>;
}

pub trait RenderStage {
    fn encode(self, core: &mut RenderCore, encoder: &mut wgpu::CommandEncoder);
}


pub struct RenderCore {
    
    pub device: &'static wgpu::Device,
    queue: wgpu::Queue,
    
    surface: wgpu::Surface,
    sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,

    pub shaders: shade::ShaderCache,

}


impl RenderCore {

    pub async fn init(window_state: &mut window::WindowState) -> Self {

        let surface = wgpu::Surface::create(window_state.window());
        
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

        let size = window_state.window().inner_size();
        let sc_desc = wgpu::SwapChainDescriptor {
            usage:  wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm, // WTF: required to some degree ?
            width:  size.width  as u32,
            height: size.height as u32,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);


        // TODO: Move this somewhere better?
        // Unfortunately we can't easily eliminate all cross-cutting
        // concerns between rendering and the window, in this case...
        window_state.init_renderer(&device, &mut queue, sc_desc.format);


        let shaders = shade::ShaderCache::new(device);

        RenderCore {
            surface,
            device,
            queue,
            sc_desc,
            shaders,
            swap_chain,
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

    #[inline]
    pub fn sequence(&mut self) -> RenderSequence {
        let encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: None,
        });

        RenderSequence {
            encoder,
            renderer: self,
        }
    }
}

pub struct RenderSequence<'r> {
    renderer: &'r mut RenderCore,
    encoder: wgpu::CommandEncoder,
}

impl RenderSequence<'_> {
    #[inline]
    pub fn draw<R: RenderStage>(mut self, r: R) -> Self {
        r.encode(&mut self.renderer, &mut self.encoder);
        self
    }

    #[inline]
    pub fn finish(self) {
        self.renderer.queue.submit(&[self.encoder.finish()]);
    }
}



#[repr(C)]
#[derive(Clone, Copy)]
struct SimpleVertex {
    position: [f32; 3],
    color:    [f32; 3],
}

unsafe impl bytemuck::Pod for SimpleVertex {}
unsafe impl bytemuck::Zeroable for SimpleVertex {}

impl Vertex for SimpleVertex {
    fn buffer_descriptor() -> wgpu::VertexBufferDescriptor<'static> {
        use wgpu::*;
        VertexBufferDescriptor {
            stride: std::mem::size_of::<SimpleVertex>() as _,
            step_mode: InputStepMode::Vertex,
            attributes: &vertex_attr_array![
                0 => Float3,
                1 => Float3
            ],
        }
    }
}



pub struct TriangleRenderer {
    vertices: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
}

impl TriangleRenderer {
    pub fn new(core: &mut RenderCore) -> Self {
        use wgpu::*;
        
        let vertex_data = &[
            SimpleVertex { position: [-1.0, -1.0, 0.0], color: [1.0, 0.0, 0.0] },
            SimpleVertex { position: [ 0.0,  1.0, 0.0], color: [0.0, 1.0, 0.0] },
            SimpleVertex { position: [ 1.0, -1.0, 0.0], color: [0.0, 0.0, 1.0] },
        ];

        let vertices = core.device.create_buffer_with_data(
            bytemuck::bytes_of(vertex_data),
            BufferUsage::VERTEX | BufferUsage::COPY_DST,
        );

        let layout_descriptor = PipelineLayoutDescriptor {
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
            
            color_states: &[
                wgpu::ColorStateDescriptor {
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
                },
            ],
            
            depth_stencil_state: None,
            
            vertex_state: VertexStateDescriptor {
                index_format: IndexFormat::Uint16,
                vertex_buffers: &[
                    SimpleVertex::buffer_descriptor(),
                ],
            },
            
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let pipeline = core.device.create_render_pipeline(&render_descriptor);

        Self {
            vertices,
            pipeline,
        }
    }
}



pub struct TriangleStage<'r, 't> {
    renderer: &'r TriangleRenderer,
    target: &'t wgpu::TextureView,
}

impl RenderStage for TriangleStage<'_, '_> {
    fn encode(self, _core: &mut RenderCore, encoder: &mut wgpu::CommandEncoder) {
        use wgpu::*;

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            depth_stencil_attachment: None,
            color_attachments: &[
                RenderPassColorAttachmentDescriptor {
                    attachment: self.target,
                    resolve_target: None,
                    load_op: LoadOp::Clear,
                    store_op: StoreOp::Store,
                    clear_color: Color::BLACK,
                }
            ],
        });

        pass.set_pipeline(&self.renderer.pipeline);
        pass.set_vertex_buffer(0, &self.renderer.vertices, 0, 0);
        pass.draw(0..3, 0..1);
    }
}

impl TriangleRenderer {
    pub fn with_target<'r, 't>(&'r self, target: &'t wgpu::TextureView) -> TriangleStage<'r, 't> {
        TriangleStage {
            target, 
            renderer: self,
        }
    }
}