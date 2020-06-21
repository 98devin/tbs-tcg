
use cgmath::prelude::*;
use cgmath::{Point3, Vector3, Matrix4};

pub mod shade;
pub mod window;
pub mod gui;


pub trait Vertex {
    fn buffer_descriptor() -> wgpu::VertexBufferDescriptor<'static>;
    fn attributes() -> &'static [wgpu::VertexAttributeDescriptor];
}

pub trait RenderStage {
    fn encode(self, core: &mut RenderCore, encoder: &mut wgpu::CommandEncoder);
}


pub struct RenderCore {
    pub device: &'static wgpu::Device,
    pub queue: wgpu::Queue,
    
    surface: wgpu::Surface,
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,

    pub shaders: shade::ShaderCache,

}


impl RenderCore {

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
        
        // WTF: We never need to deallocate this device during the program,
        // so it's not a big deal if we leak the heap reference. If necessary,
        // a Drop implementation for RenderCore could also handle this.
        let device = Box::leak(Box::new(device));

        let size = window_state.window.inner_size();
        let sc_desc = wgpu::SwapChainDescriptor {
            usage:  wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm, // WTF: for wider compatibility?
            width:  size.width  as u32,
            height: size.height as u32,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let shaders = shade::ShaderCache::new(device);

        RenderCore {
            device,
            queue,
            shaders,
            surface,
            sc_desc,
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
        self.renderer.queue.submit(
            std::iter::once(self.encoder.finish())
        );
    }
}



#[repr(C)]
#[derive(Clone, Copy)]
struct BasicVertex {
    position: Point3<f32>,
    color:    Point3<f32>,
}

unsafe impl bytemuck::Pod for BasicVertex {}
unsafe impl bytemuck::Zeroable for BasicVertex {}

impl Vertex for BasicVertex {
    fn attributes() -> &'static [wgpu::VertexAttributeDescriptor] {
        &wgpu::vertex_attr_array![
            0 => Float3,
            1 => Float3
        ]
    }

    fn buffer_descriptor() -> wgpu::VertexBufferDescriptor<'static> {
        use wgpu::*;
        VertexBufferDescriptor {
            stride: std::mem::size_of::<BasicVertex>() as _,
            step_mode: InputStepMode::Vertex,
            attributes: Self::attributes(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct BasicCamera {
    view_matrix: Matrix4<f32>,
    pos: Point3<f32>,
    dir: Vector3<f32>,
    top: Vector3<f32>,
}

unsafe impl bytemuck::Pod for BasicCamera {}
unsafe impl bytemuck::Zeroable for BasicCamera {}

impl BasicCamera {
    pub fn new(pos: Point3<f32>, dir: Vector3<f32>, top: Vector3<f32>) -> Self {
        assert_eq!(true, dir.is_perpendicular(top));
        Self {
            view_matrix: Matrix4::look_at_dir(pos, dir.normalize(), top.normalize()),
            pos, dir, top,
        }
    }

    pub fn translate(&mut self, dpos: Vector3<f32>) {
        self.pos += dpos;
        self.view_matrix.concat_self(&Matrix4::from_translation(dpos));
    }

    pub fn translate_rel(&mut self, drel: Vector3<f32>) {
        let dxt = self.dir.cross(self.top);
        self.translate(
            drel.x * dxt +
            drel.y * self.dir +
            drel.z * self.top
        );
    }

    pub fn yaw(&mut self, degrees: f32) {
        let rot = Matrix4::from_axis_angle(self.top, cgmath::Deg(degrees));
        self.dir = rot.transform_vector(self.dir);
        self.view_matrix.concat_self(&rot);
    }

    pub fn pitch(&mut self, degrees: f32) {
        let dxt = self.dir.cross(self.top);
        let rot = Matrix4::from_axis_angle(dxt, cgmath::Deg(degrees));
        self.top = rot.transform_vector(self.top);
        self.dir = rot.transform_vector(self.dir);
        self.view_matrix.concat_self(&rot);
    }

    pub fn roll(&mut self, degrees: f32) {
        let rot = Matrix4::from_axis_angle(self.dir, cgmath::Deg(degrees));
        self.top = rot.transform_vector(self.top);
        self.view_matrix.concat_self(&rot);
    }
}

    
pub trait Bindable {
    fn bind_type() -> wgpu::BindingType;
    fn bind(&self) -> wgpu::BindingResource;
}


pub struct Uniform<T> {
    buffer: wgpu::Buffer,
    data: T,
}

impl<T> std::ops::Deref for Uniform<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> std::ops::DerefMut for Uniform<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T: Sized + bytemuck::Pod> Uniform<T> {
    fn new(device: &wgpu::Device, data: T) -> Self {
        let buffer = device.create_buffer_with_data(
            bytemuck::bytes_of(&data),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST
        );

        Self {
            buffer, data,
        }
    }

    fn refresh(&self, core: &RenderCore) {
        core.queue.write_buffer(
            &self.buffer,
            0 as wgpu::BufferAddress,
            bytemuck::bytes_of(&self.data)
        );
    }
}

impl<T: Sized> Bindable for Uniform<T> {
    fn bind_type() -> wgpu::BindingType {
        wgpu::BindingType::UniformBuffer {
            dynamic: false,
            min_binding_size: std::num::NonZeroU64::new(
                std::mem::size_of::<T>() as u64
            ),
        }
    }

    fn bind(&self) -> wgpu::BindingResource {
        wgpu::BindingResource::Buffer(self.buffer.slice(..))
    }
}



pub struct BasicRenderer {
    // camera: Uniform<BasicCamera>,
    vertices: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
}

impl BasicRenderer {
    pub fn new(core: &mut RenderCore) -> Self {
        use wgpu::*;
        
        let vertex_data = &[
            BasicVertex { position: Point3::new(-1.0, -1.0, 0.0), color: Point3::new(1.0, 0.0, 0.0) },
            BasicVertex { position: Point3::new( 1.0, -1.0, 0.0), color: Point3::new(0.0, 0.0, 1.0) },
            BasicVertex { position: Point3::new( 0.0,  1.0, 0.0), color: Point3::new(0.0, 1.0, 0.0) },
        ];

        let vertices = core.device.create_buffer_with_data(
            bytemuck::bytes_of(vertex_data),
            BufferUsage::VERTEX | BufferUsage::COPY_DST,
        );

        let layout_descriptor = PipelineLayoutDescriptor {
            bind_group_layouts: &[]
        };

        let layout = core.device.create_pipeline_layout(&layout_descriptor);

        let vert_module = core.shaders.load("basic.vert");
        let frag_module = core.shaders.load("basic.frag");

        let render_descriptor = RenderPipelineDescriptor {
            layout: &layout,
            
            vertex_stage: vert_module.descriptor(),
            fragment_stage: Some(frag_module.descriptor()),
            
            rasterization_state: Some(RasterizationStateDescriptor {
                front_face: FrontFace::Ccw,
                cull_mode: CullMode::Back,
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
                    BasicVertex::buffer_descriptor(),
                ],
            },
            
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let pipeline = core.device.create_render_pipeline(&render_descriptor);

        BasicRenderer {
            vertices,
            pipeline,
        }
    }
}



pub struct BasicStage<'r, 't> {
    pub basic_renderer: &'r BasicRenderer,
    pub render_target: &'t wgpu::TextureView,
}

impl RenderStage for BasicStage<'_, '_> {
    fn encode(self, _core: &mut RenderCore, encoder: &mut wgpu::CommandEncoder) {
        use wgpu::*;

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            depth_stencil_attachment: None,
            color_attachments: &[
                RenderPassColorAttachmentDescriptor {
                    attachment: self.render_target,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: true,
                    },
                }
            ],
        });

        pass.set_pipeline(&self.basic_renderer.pipeline);
        pass.set_vertex_buffer(0, self.basic_renderer.vertices.slice(..));
        pass.draw(0..3, 0..1);
    }
}
