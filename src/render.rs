
use nalgebra_glm as glm;

pub mod window;
pub mod gui;
pub mod cache;
pub mod core;
pub mod camera;

pub use self::core::*;
pub use self::cache::{
    shaders::ShaderCache,
    textures::TextureCache,
    models::ModelCache,
};

use crate::util::{self, bytes};


pub trait Resource<'p> {
    /// information suitable for describing this resource's structure
    type Descriptor: 'p; 

    /// the data the resource represents
    type Handle: 'p;
}

pub type Descriptor<'p, R> = <R as Resource<'p>>::Descriptor;
pub type Handle<'p, R> = <R as Resource<'p>>::Handle;


macro_rules! tuple_impls {
    () => { };
    ($t:ident, $($ts:ident,)*) => {
        impl<'p, $t: Resource<'p>, $($ts: Resource<'p>,)*> Resource<'p> for ($t, $($ts,)*) {
            type Descriptor = (Descriptor<'p, $t>, $(Descriptor<'p, $ts>,)*);
            type Handle = (Handle<'p, $t>, $(Handle<'p, $ts>,)*);
        }
        tuple_impls!($($ts,)*);
    }
}

tuple_impls!(A, B, C, D, E, F,);


/// A resource to use just to indicate an extra regular parameter is necessary.
/// For example, `Core`, or `winit::dpi::PhysicalSize<u32>`.
pub struct With<T>(std::marker::PhantomData<T>);

impl<'p, T: 'p> Resource<'p> for With<T> {
    type Descriptor = T;
    type Handle = T;
}

/// Equivalent to With<()>, but clearer in intent, perhaps.
impl Resource<'_> for () {
    type Descriptor = ();
    type Handle = ();
}

impl<'r, R: Resource<'r>> Resource<'r> for &'r R {
    type Descriptor = &'r Descriptor<'r, R>;
    type Handle = &'r Handle<'r, R>;
}

impl<'r, R: Resource<'r>> Resource<'r> for &'r mut R {
    type Descriptor = &'r mut Descriptor<'r, R>;
    type Handle = &'r mut Handle<'r, R>;
}

pub struct Borrow<R>(std::marker::PhantomData<*const R>);

impl<'r, R: Resource<'r>> Resource<'r> for Borrow<R> {
    type Descriptor = util::Borrow<'r, Descriptor<'r, R>>;
    type Handle = util::Borrow<'r, Handle<'r, R>>;
}



pub trait Pass<'p>: Sized {
    type Input:  Resource<'p>;
    type Output: Resource<'p>;

    type Config;
    type Params;
    
    fn construct(config: Self::Config, input: InputDesc<'p, Self>) -> (Self, OutputDesc<'p, Self>);
    fn perform(self: &'p mut Self, params: Self::Params, input: InputHandle<'p, Self>) -> OutputHandle<'p, Self>;

    /// Should be overridden if there is a more efficient way
    /// to modify the pass only slightly.
    fn refresh(self: &'p mut Self, config: Self::Config, input: InputDesc<'p, Self>) -> OutputDesc<'p, Self>
    {
        let (new_self, output) = Self::construct(config, input);
        *self = new_self;
        output
    }
}

pub type Input<'p, P> = <P as Pass<'p>>::Input;
pub type Output<'p, P> = <P as Pass<'p>>::Output;

pub type InputDesc<'p, P> = Descriptor<'p, Input<'p, P>>;
pub type OutputDesc<'p, P> = Descriptor<'p, Output<'p, P>>;

pub type InputHandle<'p, P> = Handle<'p, Input<'p, P>>;
pub type OutputHandle<'p, P> = Handle<'p, Output<'p, P>>;


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
    pub fn new(device: &wgpu::Device, data: T) -> Self {
        let buffer = device.create_buffer_with_data(
            bytes::of(&data),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST
        );

        Self {
            buffer, data,
        }
    }

    fn refresh(&self, core: &Core) {
        core.queue.write_buffer(
            &self.buffer,
            0 as wgpu::BufferAddress,
            bytes::of(&self.data)
        );
    }

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


impl<'p, T: Sized + bytes::IntoBytes> Resource<'p> for Uniform<T> {
    type Descriptor = wgpu::BindGroupLayout;
    type Handle = wgpu::BindGroup;
}

impl<'p> Resource<'p> for wgpu::SwapChainFrame {
    type Descriptor = wgpu::SwapChainDescriptor;
    type Handle = wgpu::TextureView;
}

impl<'p> Resource<'p> for wgpu::Texture {
    type Descriptor = wgpu::TextureDescriptor<'p>;
    type Handle = wgpu::Texture;
}

impl<'p> Resource<'p> for wgpu::TextureView {
    type Descriptor = wgpu::TextureDescriptor<'p>;
    type Handle = wgpu::TextureView;
}

impl<'p> Resource<'p> for wgpu::Sampler {
    type Descriptor = wgpu::SamplerDescriptor<'p>;
    type Handle = wgpu::Sampler;
}


pub struct AnyAttachment;

pub enum AnyAttachmentDescriptor<'p> {
    TextureView(&'p wgpu::TextureDescriptor<'p>),
    SwapChain(&'p wgpu::SwapChainDescriptor),
}

impl AnyAttachmentDescriptor<'_> {
    #[inline]
    pub fn width(&self) -> u32 {
        match self {
            AnyAttachmentDescriptor::TextureView(tview) => tview.size.width,
            AnyAttachmentDescriptor::SwapChain(schain) => schain.width,
        }
    }

    #[inline]
    pub fn height(&self) -> u32 {
        match self {
            AnyAttachmentDescriptor::TextureView(tview) => tview.size.height,
            AnyAttachmentDescriptor::SwapChain(schain) => schain.height,
        }
    }

    #[inline]
    pub fn format(&self) -> wgpu::TextureFormat {
        match self {
            AnyAttachmentDescriptor::TextureView(tview) => tview.format,
            AnyAttachmentDescriptor::SwapChain(schain) => schain.format,
        }
    }
}


impl<'p> Resource<'p> for AnyAttachment {
    type Descriptor = AnyAttachmentDescriptor<'p>;
    type Handle = &'p wgpu::TextureView;
}



pub struct BasicPass {
    pub camera: Uniform<camera::GimbalCamera>,
    pub project: Uniform<glm::Mat4>,
    u_cam_group: wgpu::BindGroup,
    u_tex_group: wgpu::BindGroup,
    pub pipeline: wgpu::RenderPipeline,
    pub zbuffer: wgpu::Texture,
}

impl<'p> Pass<'p> for BasicPass {
    
    type Input =
        ( With<&'p Core>
        , AnyAttachment // color attachment
        );
    
    type Output =
        Borrow<wgpu::Texture>; // depth buffer

    type Params =
        ();

    type Config =
        ();

    fn construct(_: (), input: InputDesc<'p, Self>) -> (Self, OutputDesc<'p, Self>) {
        let (core, target) = input;

        let camera = Uniform::new(core.device, camera::GimbalCamera::new(
            glm::vec3(0.0,  0.0, -5.0),
            glm::vec3(0.0,  0.0,  0.0),
            glm::vec3(0.0, -1.0,  0.0),
        ));

        let project = Uniform::new(core.device, 
            glm::perspective_fov_lh_zo(
                120.0, 
                target.width() as f32, 
                target.height() as f32, 
                1.0, 
                100.0,
            ),
        );

        let zbuffer_desc = wgpu::TextureDescriptor {
            label: Some("BasicRenderer depth buffer"),
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            mip_level_count: 1,
            sample_count: 1,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
            size: wgpu::Extent3d {
                width: target.width(),
                height: target.height(),
                depth: 1,
            },
        };

        let zbuffer = core.device.create_texture(&zbuffer_desc);

        let u_cam_descriptor = wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera uniform"),
            bindings: &[
                wgpu::BindGroupLayoutEntry::new(
                    0, wgpu::ShaderStage::all(),
                    Uniform::<camera::GimbalCamera>::bind_type(),
                ),
                wgpu::BindGroupLayoutEntry::new(
                    1, wgpu::ShaderStage::all(),
                    Uniform::<glm::Mat4>::bind_type(),
                ),
            ],
        };

        let u_cam_layout = core.device.create_bind_group_layout(&u_cam_descriptor);

        let u_cam_bind_desc = wgpu::BindGroupDescriptor {
            label: Some("Camera uniform"),
            layout: &u_cam_layout,
            bindings: &[
                wgpu::Binding { binding: 0, resource: camera.bind() },
                wgpu::Binding { binding: 1, resource: project.bind() },
            ],
        };

        let u_cam_group = core.device.create_bind_group(&u_cam_bind_desc);

        let tex = core.textures.load("gray_marble.tif");

        let u_tex_group = core.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture uniform"),
            layout: &tex.bind_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&tex.texture.create_default_view()),
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&tex.sampler),
                },
            ],
        });

        let layout_descriptor = wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&u_cam_layout, &tex.bind_layout],
        };

        let layout = core.device.create_pipeline_layout(&layout_descriptor);

        let vert_module = core.shaders.load("basic.vert");
        let frag_module = core.shaders.load("basic.frag");

        let render_descriptor = wgpu::RenderPipelineDescriptor {
            layout: &layout,
            
            vertex_stage: vert_module.descriptor(),
            fragment_stage: Some(frag_module.descriptor()),
            
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Cw,
                cull_mode: wgpu::CullMode::Back,
                ..Default::default()
            }),
            
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            
            color_states: &[
                wgpu::ColorStateDescriptor {
                    format: target.format(),
                    color_blend: wgpu::BlendDescriptor {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha_blend: wgpu::BlendDescriptor {
                        src_factor: wgpu::BlendFactor::OneMinusDstAlpha,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    write_mask: wgpu::ColorWrite::ALL,
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
            
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint32,
                vertex_buffers: &[
                    // positions
                    wgpu::VertexBufferDescriptor {
                        attributes: &wgpu::vertex_attr_array![0 => Float3],
                        step_mode: wgpu::InputStepMode::Vertex,
                        stride: wgpu::vertex_format_size!(Float3),
                    }, 
                    // texcoords
                    wgpu::VertexBufferDescriptor {
                        attributes: &wgpu::vertex_attr_array![1 => Float2],
                        step_mode: wgpu::InputStepMode::Vertex,
                        stride: wgpu::vertex_format_size!(Float2),
                    },
                    // normals
                    wgpu::VertexBufferDescriptor {
                        attributes: &wgpu::vertex_attr_array![2 => Float3],
                        step_mode: wgpu::InputStepMode::Vertex,
                        stride: wgpu::vertex_format_size!(Float3),
                    },
                ],
            },
            
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let pipeline = core.device.create_render_pipeline(&render_descriptor);

        let pass = Self {
            camera,
            project,
            u_cam_group,
            u_tex_group,
            pipeline,
            zbuffer,
        };

        (pass, zbuffer_desc.into())
    }


    fn perform(self: &'p mut Self, _: (), input: InputHandle<'p, Self>) -> OutputHandle<'p, Self> {
        let (core, target) = input;

        let model = core.models.load(cache::models::ModelName {
            file: "torus.obj",
            name: "Torus", // FIXME: This is a _terrible_ name...
        });

        self.camera.refresh(core);
        self.project.refresh(core);


        let zbuffer_view = self.zbuffer.create_default_view();

        let mut encoder = core.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Basic Pass"),
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[
                wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(
                            wgpu::Color { r: 0.2, g: 0.2, b: 0.2, a: 1.0 }
                        ),
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

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.u_cam_group, &[]);
        pass.set_bind_group(1, &self.u_tex_group, &[]);
        
        pass.set_index_buffer(model.indices.slice(..));
        pass.set_vertex_buffer(0, model.positions.slice(..));
        pass.set_vertex_buffer(1, model.texcoords.as_ref().unwrap().slice(..));
        pass.set_vertex_buffer(2, model.normals.as_ref().unwrap().slice(..));

        pass.draw_indexed(0..model.vertex_ct, 0, 0..1);

        drop(pass); // end borrow

        core.queue.submit(std::iter::once(
            encoder.finish()
        ));

        (&self.zbuffer).into()
    }
}


/// The pass which constructs/holds an appropriately-sized render target texture,
/// independent of the resulting screen size (which is filled by PostPass). 
pub struct PrePass {
    pub hdr_texture: wgpu::Texture,
}

impl<'p> Pass<'p> for PrePass {

    type Input = &'p wgpu::SwapChainFrame;
    type Output = Borrow<wgpu::TextureView>;

    type Config = (&'p Core, f64); // scaling ratio 
    type Params = ();


    fn construct(config: Self::Config, input: InputDesc<'p, Self>) -> (Self, OutputDesc<'p, Self>) {
        let (core, scale) = config;
        let schain = input;

        let hdr_texture_desc = wgpu::TextureDescriptor {
            label: Some("Main render target"),
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            mip_level_count: 1,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
            sample_count: 1,
            size: wgpu::Extent3d {
                width: (schain.width as f64 * scale) as u32,
                height: (schain.height as f64 * scale) as u32,
                depth: 1,
            },
        };

        let hdr_texture = core.device.create_texture(&hdr_texture_desc);

        let pass = Self { hdr_texture };

        (pass, hdr_texture_desc.into())
    }

    fn perform(self: &'p mut Self, _: (), _: InputHandle<'p, Self>) -> OutputHandle<'p, Self> {
        self.hdr_texture.create_default_view().into()
    }

}



pub struct PostPass {
    pipeline: wgpu::RenderPipeline,
    tex_group: wgpu::BindGroup,
}

impl<'p> Pass<'p> for PostPass {

    type Input = (With<&'p Core>, &'p wgpu::SwapChainFrame);
    type Output = ();

    type Config = util::Borrow<'p, wgpu::Texture>;
    type Params = ();

    fn perform(self: &'p mut Self, _: (), input: InputHandle<'p, Self>) -> OutputHandle<'p, Self> {
        let (core, target) = input;

        let mut encoder = core.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Postpass"),
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            depth_stencil_attachment: None,
            color_attachments: &[
                wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                },
            ],
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.tex_group, &[]);
        pass.draw(0..3, 0..1);

        drop(pass); // end borrow

        core.queue.submit(std::iter::once(
            encoder.finish()
        ));
    }
    
    fn construct(texture: Self::Config, input: InputDesc<'p, Self>) -> (Self, OutputDesc<'p, Self>) {
        let (core, schain) = input;

        let tex_layout = core.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Postpass input texture"),
            bindings: &[
                wgpu::BindGroupLayoutEntry::new(
                    0, wgpu::ShaderStage::FRAGMENT,
                    wgpu::BindingType::SampledTexture {
                        dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                        component_type: wgpu::TextureComponentType::Float,
                    },
                ),
                wgpu::BindGroupLayoutEntry::new(
                    1, wgpu::ShaderStage::FRAGMENT,
                    wgpu::BindingType::Sampler { comparison: false },
                ),
            ],
        });

        let sample_desc = wgpu::SamplerDescriptor {
            label: Some("Postpass sampler"),  
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: None,
            ..Default::default()
        };

        let sampler = core.device.create_sampler(&sample_desc);

        let tex_group = core.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Postpass bind group"),
            layout: &tex_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.create_default_view()),
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                }
            ],
        });


        let layout = core.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[&tex_layout],
            },
        );

        let vert_module = core.shaders.load("post.vert");
        let frag_module = core.shaders.load("post.frag");

        let render_desc = wgpu::RenderPipelineDescriptor {
            layout: &layout,
            
            vertex_stage: vert_module.descriptor(),
            fragment_stage: Some(frag_module.descriptor()),
            
            rasterization_state: Some(Default::default()),
            
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            
            color_states: &[
                wgpu::ColorStateDescriptor {
                    format: schain.format,
                    color_blend: wgpu::BlendDescriptor {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha_blend: wgpu::BlendDescriptor {
                        src_factor: wgpu::BlendFactor::OneMinusDstAlpha,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    write_mask: wgpu::ColorWrite::ALL,
                },
            ],
            
            depth_stencil_state: None,

            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[]
            },
            
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let pipeline = core.device.create_render_pipeline(&render_desc);

        let pass = Self {
            pipeline,
            tex_group,
        };

        (pass, ())
    }

}



pub struct MainPass {
    pub pre: PrePass,
    pub basic: BasicPass,
    pub post: PostPass,
}


impl<'p> Pass<'p> for MainPass {

    type Input  = (With<&'p Core>, &'p wgpu::SwapChainFrame);
    type Output = &'p wgpu::SwapChainFrame;    

    type Config = f64; // scaling ratio for render quality
    type Params = ();

    fn construct(config: Self::Config, input: InputDesc<'p, Self>) -> (Self, OutputDesc<'p, Self>) {

        let (core, schain) = input;
        let scale = config;

        let (pre, hdr_target) = PrePass::construct((core, scale), schain);

        let (basic, _) = BasicPass::construct((), (core, AnyAttachmentDescriptor::TextureView(&hdr_target)));

        let (post, ()) = PostPass::construct((&pre.hdr_texture).into(), (core, schain));

        let pass = Self {
            pre, basic, post,
        };

        (pass, schain)
    }
    
    fn perform(self: &'p mut Self, _: (), input: InputHandle<'p, Self>) -> OutputHandle<'p, Self> {
        
        let (core, schain) = input;
        
        let hdr_view = self.pre.perform((), schain);

        let _ = self.basic.perform((), (core, &hdr_view));

        let () = self.post.perform((), (core, schain));

        schain
    }

}