
use nalgebra_glm as glm;

pub mod window;
pub mod gui;
pub mod bytes;
pub mod cache;

pub trait Vertex {
    fn buffer_descriptor() -> wgpu::VertexBufferDescriptor<'static>;
    fn attributes() -> &'static [wgpu::VertexAttributeDescriptor];
}

pub trait RenderStage {
    fn encode(self, core: &mut RenderCore, encoder: &mut wgpu::CommandEncoder);
}



use cache::{
    shaders::ShaderCache,
    textures::TextureCache,
    models::ModelCache,
};


pub struct RenderCore {
    pub device: &'static wgpu::Device,
    pub queue: &'static wgpu::Queue,
    
    surface: wgpu::Surface,
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,

    pub shaders: ShaderCache,
    pub textures: TextureCache,
    pub models: ModelCache,
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

        RenderCore {
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
#[derive(Copy, Clone, Debug)]
pub struct GimbalCamera {
    view: glm::Mat4,
    pos:    glm::Vec3,
    center: glm::Vec3,
    dir:    glm::Vec3,
    top:    glm::Vec3,
}

unsafe impl bytes::IntoBytes for GimbalCamera {}

impl GimbalCamera {
    pub fn new(pos: glm::Vec3, center: glm::Vec3, top: glm::Vec3) -> Self {
        let dir = (center - pos).normalize();
        GimbalCamera {
            view: glm::look_at_lh(&pos, &center, &top),
            pos, center, dir, top,
        }
    }

    fn refresh_view_matrix(&mut self) {
        self.view = glm::look_at_lh(&self.pos, &(self.pos + self.dir), &self.top);
    }

    pub fn translate(&mut self, dpos: glm::Vec3) {
        self.pos += dpos;
        self.refresh_view_matrix();
    }

    pub fn translate_rel(&mut self, drel: glm::Vec3) {
        let dxt = self.dir.cross(&self.top);
        self.translate(
            drel.x * dxt +
            drel.y * self.top +
            drel.z * self.dir
        );
    }

    pub fn zoom(&mut self, ratio: f32) {
        self.pos = glm::lerp(&self.pos, &self.center, ratio);
        self.refresh_view_matrix();
    }

    pub fn gimbal_ud(&mut self, degrees: f32) {
        let dxt = self.dir.cross(&self.top);
        let rot = glm::rotation(degrees, &dxt.normalize());
        self.top = rot.transform_vector(&(self.top - self.center)) + self.center;
        self.pos = rot.transform_vector(&(self.pos - self.center)) + self.center;
        self.dir = rot.transform_vector(&(self.dir - self.center)) + self.center;
        self.refresh_view_matrix();
    }

    pub fn gimbal_lr(&mut self, degrees: f32) {
        let rot = glm::rotation(degrees, &self.top);
        self.top = rot.transform_vector(&(self.top - self.center)) + self.center;
        self.pos = rot.transform_vector(&(self.pos - self.center)) + self.center;
        self.dir = rot.transform_vector(&(self.dir - self.center)) + self.center;
        self.refresh_view_matrix();
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

impl<T: Sized + bytes::IntoBytes> Uniform<T> {
    fn new(device: &wgpu::Device, data: T) -> Self {
        let buffer = device.create_buffer_with_data(
            bytes::of(&data),
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
            bytes::of(&self.data)
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
    device: &'static wgpu::Device,

    pub camera: Uniform<GimbalCamera>,
    pub project: Uniform<glm::Mat4>,
    u_cam_group: wgpu::BindGroup,
    u_tex_group: wgpu::BindGroup,
    // vertices: wgpu::Buffer,
    // indices: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub zbuffer: wgpu::Texture,
}

impl BasicRenderer {
    pub fn adjust_screen_res(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        *self.project = glm::perspective_fov_lh_zo(
            120.0, 
            size.width as f32, 
            size.height as f32, 
            1.0, 
            100.0,
        );

        self.zbuffer = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("BasicRenderer depth buffer"),
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            mip_level_count: 1,
            sample_count: 1,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth: 1,
            },
        });
    }

    pub fn new(core: &mut RenderCore) -> Self {
        use wgpu::*;

        let camera = Uniform::new(core.device, GimbalCamera::new(
            glm::vec3(0.0,  0.0, -5.0),
            glm::vec3(0.0,  0.0,  0.0),
            glm::vec3(0.0, -1.0,  0.0),
        ));

        let project = Uniform::new(core.device, 
            glm::perspective_fov_lh_zo(
                120.0, 
                core.sc_desc.width as f32, 
                core.sc_desc.height as f32, 
                1.0, 
                100.0,
            ),
        );

        let zbuffer = core.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("BasicRenderer depth buffer"),
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            mip_level_count: 1,
            sample_count: 1,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
            size: wgpu::Extent3d {
                width: core.sc_desc.width,
                height: core.sc_desc.height,
                depth: 1,
            },
        });


        let u_cam_descriptor = BindGroupLayoutDescriptor {
            label: Some("Camera uniform"),
            bindings: &[
                BindGroupLayoutEntry::new(
                    0, ShaderStage::VERTEX,
                    Uniform::<GimbalCamera>::bind_type(),
                ),
                BindGroupLayoutEntry::new(
                    1, ShaderStage::VERTEX,
                    Uniform::<glm::Mat4>::bind_type(),
                ),
            ],
        };

        let u_cam_layout = core.device.create_bind_group_layout(&u_cam_descriptor);

        let u_cam_bind_desc = BindGroupDescriptor {
            label: Some("Camera uniform"),
            layout: &u_cam_layout,
            bindings: &[
                Binding { binding: 0, resource: camera.bind() },
                Binding { binding: 1, resource: project.bind() },
            ],
        };

        let u_cam_group = core.device.create_bind_group(&u_cam_bind_desc);

        let tex = core.textures.load("gray_marble.tif");

        let u_tex_group = core.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Texture uniform"),
            layout: &tex.bind_layout,
            bindings: &[
                Binding {
                    binding: 0,
                    resource: BindingResource::TextureView(&tex.texture.create_default_view()),
                },
                Binding {
                    binding: 1,
                    resource: BindingResource::Sampler(&tex.sampler),
                },
            ],
        });

        let layout_descriptor = PipelineLayoutDescriptor {
            bind_group_layouts: &[&u_cam_layout, &tex.bind_layout],
        };

        let layout = core.device.create_pipeline_layout(&layout_descriptor);

        let vert_module = core.shaders.load("basic.vert");
        let frag_module = core.shaders.load("basic.frag");

        let render_descriptor = RenderPipelineDescriptor {
            layout: &layout,
            
            vertex_stage: vert_module.descriptor(),
            fragment_stage: Some(frag_module.descriptor()),
            
            rasterization_state: Some(RasterizationStateDescriptor {
                front_face: FrontFace::Cw,
                cull_mode: CullMode::Back,
                ..Default::default()
            }),
            
            primitive_topology: PrimitiveTopology::TriangleList,
            
            color_states: &[
                ColorStateDescriptor {
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
            
            depth_stencil_state: Some(
                wgpu::DepthStencilStateDescriptor {
                    depth_compare: wgpu::CompareFunction::Less,
                    depth_write_enabled: true,
                    format: wgpu::TextureFormat::Depth24Plus,
                    stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
                    stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
                    stencil_read_mask: 0,
                    stencil_write_mask: 0,
                }
            ),
            
            vertex_state: VertexStateDescriptor {
                index_format: IndexFormat::Uint32,
                vertex_buffers: &[
                    // positions
                    VertexBufferDescriptor {
                        attributes: &vertex_attr_array![0 => Float3],
                        step_mode: InputStepMode::Vertex,
                        stride: vertex_format_size!(Float3),
                    }, 
                    // texcoords
                    VertexBufferDescriptor {
                        attributes: &vertex_attr_array![1 => Float2],
                        step_mode: InputStepMode::Vertex,
                        stride: vertex_format_size!(Float2),
                    },
                ],
            },
            
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let pipeline = core.device.create_render_pipeline(&render_descriptor);

        BasicRenderer {
            device: core.device,
            camera,
            project,
            u_cam_group,
            u_tex_group,
            pipeline,
            zbuffer,
        }
    }
}



pub struct BasicStage<'r, 't> {
    pub basic_renderer: &'r BasicRenderer,
    pub render_target: &'t wgpu::TextureView,
}

impl RenderStage for BasicStage<'_, '_> {
    fn encode(self, core: &mut RenderCore, encoder: &mut wgpu::CommandEncoder) {
        use wgpu::*;
        
        let model = core.models.load(cache::models::ModelName {
            file: "torus.obj",
            name: "Torus", // FIXME: This is a _terrible_ name...
        });

        self.basic_renderer.camera.refresh(core);
        self.basic_renderer.project.refresh(core);


        let zbuffer_view = self.basic_renderer.zbuffer.create_default_view();

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[
                RenderPassColorAttachmentDescriptor {
                    attachment: self.render_target,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color { r: 0.2, g: 0.2, b: 0.2, a: 1.0 }),
                        store: true,
                    },
                }
            ],
            depth_stencil_attachment: Some(
                wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &zbuffer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }
            )
        });

        pass.set_pipeline(&self.basic_renderer.pipeline);
        pass.set_bind_group(0, &self.basic_renderer.u_cam_group, &[]);
        pass.set_bind_group(1, &self.basic_renderer.u_tex_group, &[]);
        
        pass.set_index_buffer(model.indices.slice(..));
        pass.set_vertex_buffer(0, model.positions.slice(..));
        pass.set_vertex_buffer(1, model.texcoords.as_ref().unwrap().slice(..));

        pass.draw_indexed(0..model.vertex_ct, 0, 0..1);
    }
}
