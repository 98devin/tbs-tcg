
use imgui::{
    Context, 
    DrawCmd::Elements, 
    DrawData, 
    DrawIdx, 
    DrawList, 
    DrawVert, 
    TextureId, 
    Textures,
};

use wgpu::*;
use crate::render::{self, gui, window, bytes};

/// A container for a bindable texture to be used internally.
pub struct ImguiTexture {
    bind_group: BindGroup,
}

impl ImguiTexture {
    /// Creates a new imgui texture from a wgpu texture.
    pub fn new(texture: wgpu::Texture, layout: &BindGroupLayout, device: &Device) -> Self {
        // Extract the texture view.
        let view = texture.create_default_view();

        // Create the texture sampler.
        let sampler = device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare: None, // Some(wgpu::CompareFunction::Always),
            ..Default::default()
        });

        // Create the texture bind group from the layout.
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout,
            bindings: &[
                Binding {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                Binding {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        ImguiTexture { bind_group }
    }
}

pub struct ImguiRenderer {
    pipeline: RenderPipeline,
    uniform_buffer: Buffer,
    uniform_bind_group: BindGroup,
    textures: Textures<ImguiTexture>,
    texture_layout: BindGroupLayout,
    clear_color: Option<Color>,
    index_buffers: Vec<Buffer>,
    vertex_buffers: Vec<Buffer>,
}

impl ImguiRenderer {
  
    /// Create an entirely new imgui wgpu renderer.
    pub fn new(
        core: &mut render::RenderCore,
        format: TextureFormat,
        clear_color: Option<Color>,
    ) -> Self {
        
        // Load shaders.
        let vs_module = core.shaders.load("imgui.vert");
        let fs_module = core.shaders.load("imgui.frag");

        // Create the uniform matrix buffer.
        let size = 64;
        let uniform_buffer = core.device.create_buffer(&BufferDescriptor {
            label: None,
            size,
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        // Create the uniform matrix buffer bind group layout.
        let uniform_layout = core.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            bindings: &[BindGroupLayoutEntry::new(
                0,
                wgpu::ShaderStage::VERTEX,
                BindingType::UniformBuffer {
                    dynamic: false,
                    min_binding_size: std::num::NonZeroU64::new(64),
                },
            )],
        });

        // Create the uniform matrix buffer bind group.
        let uniform_bind_group = core.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &uniform_layout,
            bindings: &[Binding {
                binding: 0,
                resource: BindingResource::Buffer(uniform_buffer.slice(..))
            }],
        });

        // Create the texture layout for further usage.
        let texture_layout = core.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            bindings: &[
                BindGroupLayoutEntry::new(
                    0, wgpu::ShaderStage::FRAGMENT,
                    BindingType::SampledTexture {
                        multisampled: false,
                        component_type: TextureComponentType::Float,
                        dimension: TextureViewDimension::D2,
                    },
                ),
                BindGroupLayoutEntry::new(
                    1, wgpu::ShaderStage::FRAGMENT,
                    BindingType::Sampler { comparison: false },
                ),
            ],
        });

        // Create the render pipeline layout.
        let pipeline_layout = core.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            bind_group_layouts: &[&uniform_layout, &texture_layout],
        });

        // Create the render pipeline.
        let pipeline = core.device.create_render_pipeline(&RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: vs_module.descriptor(),
            fragment_stage: Some(fs_module.descriptor()),
            rasterization_state: Some(RasterizationStateDescriptor {
                front_face: FrontFace::Cw,
                cull_mode: CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: PrimitiveTopology::TriangleList,
            color_states: &[ColorStateDescriptor {
                format,
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
                vertex_buffers: &[VertexBufferDescriptor {
                    stride: std::mem::size_of::<DrawVert>() as _,
                    step_mode: InputStepMode::Vertex,
                    attributes: &vertex_attr_array![
                        0 => Float2,
                        1 => Float2,
                        2 => Uchar4
                    ],
                }],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            textures: Textures::new(),
            texture_layout,
            clear_color,
            vertex_buffers: vec![],
            index_buffers: vec![],
        }
    }

    /// Render the current imgui frame.
    pub fn render<'r>(
        &'r mut self,
        draw_data: &DrawData,
        core: &mut render::RenderCore,
        encoder: &'r mut CommandEncoder,
        view: &TextureView,
    ) {
        // FIXME: I don't think this framebuffer size check actually did anything,
        // so I swapped it for checking the logical width/height. Maybe replace it
        // with something which casts to an integer...?
        //
        // let fb_width = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
        // let fb_height = draw_data.display_size[1] * draw_data.framebuffer_scale[1];
        
        let width = draw_data.display_size[0];
        let height = draw_data.display_size[1];
        
        // If the render area is <= 0, exit.
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        
        // Create and update the transform matrix for the current frame.
        let matrix = [
            [2.0 / width, 0.0, 0.0, 0.0],
            [0.0, 2.0 / -height as f32, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0, 1.0],
        ];
        core.queue.write_buffer(&self.uniform_buffer, 0, bytes::of(&matrix));

        // Start a new renderpass and prepare it properly.
        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[RenderPassColorAttachmentDescriptor {
                attachment: &view,
                resolve_target: None,
                ops: Operations {
                    load: self.clear_color.map(LoadOp::Clear).unwrap_or(LoadOp::Load),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.uniform_bind_group, &[]);

        self.vertex_buffers.clear();
        self.index_buffers.clear();

        // FIXME: We create several buffers per frame this way!
        // Surely we can consolidate these using draw_data.total_{idx,vtx}_count!
        for draw_list in draw_data.draw_lists() {
            self.vertex_buffers
                .push(self.upload_vertex_buffer(core.device, draw_list.vtx_buffer()));
            self.index_buffers
                .push(self.upload_index_buffer(core.device, draw_list.idx_buffer()));
        }

        // Execute all the imgui render work.
        for (draw_list_buffers_index, draw_list) in draw_data.draw_lists().enumerate() {
            self.render_draw_list(
                &mut rpass,
                &draw_list,
                draw_data.display_pos,
                draw_data.framebuffer_scale,
                draw_list_buffers_index,
            );
        }
    }

    /// Render a given `DrawList` from imgui onto a wgpu frame.
    fn render_draw_list<'render>(
        &'render self,
        rpass: &mut RenderPass<'render>,
        draw_list: &DrawList,
        clip_off: [f32; 2],
        clip_scale: [f32; 2],
        draw_list_buffers_index: usize,
    ) {
        let mut start = 0;

        let index_buffer = &self.index_buffers[draw_list_buffers_index];
        let vertex_buffer = &self.vertex_buffers[draw_list_buffers_index];

        // Make sure the current buffers are attached to the render pass.
        rpass.set_index_buffer(index_buffer.slice(..));
        rpass.set_vertex_buffer(0, vertex_buffer.slice(..));

        for cmd in draw_list.commands() {
            if let Elements { count, cmd_params } = cmd {
                let clip_rect = [
                    (cmd_params.clip_rect[0] - clip_off[0]) * clip_scale[0],
                    (cmd_params.clip_rect[1] - clip_off[1]) * clip_scale[1],
                    (cmd_params.clip_rect[2] - clip_off[0]) * clip_scale[0],
                    (cmd_params.clip_rect[3] - clip_off[1]) * clip_scale[1],
                ];

                // Set the current texture bind group on the renderpass.
                let tex = self
                    .textures
                    .get(cmd_params.texture_id)
                    .unwrap();

                rpass.set_bind_group(1, &tex.bind_group, &[]);

                // Set scissors on the renderpass.
                rpass.set_scissor_rect(
                    clip_rect[0].max(0.0).floor() as u32,
                    clip_rect[1].max(0.0).floor() as u32,
                    (clip_rect[2] - clip_rect[0]).abs().ceil() as u32,
                    (clip_rect[3] - clip_rect[1]).abs().ceil() as u32,
                );

                // Draw the current batch of vertices with the renderpass.
                let end = start + count as u32;
                rpass.draw_indexed(start..end, 0, 0..1);
                start = end;
            }
        }
    }

    /// Upload the vertex buffer to the gPU.
    fn upload_vertex_buffer(&self, device: &Device, vertices: &[DrawVert]) -> Buffer {
        let data = bytes::of_slice(vertices);
        device.create_buffer_with_data(data, BufferUsage::VERTEX)
    }

    /// Upload the index buffer to the GPU.
    fn upload_index_buffer(&self, device: &Device, indices: &[DrawIdx]) -> Buffer {
        let data = bytes::of_slice(indices);
        device.create_buffer_with_data(data, BufferUsage::INDEX)
    }

    /// Updates the texture on the GPU corresponding to the current imgui font atlas.
    ///
    /// This has to be called after loading a font.
    pub fn build_font_texture(&mut self, core: &mut render::RenderCore, imgui: &mut Context) {
        let mut atlas = imgui.fonts();
        let handle = atlas.build_rgba32_texture();
        let font_texture_id =
            self.upload_texture(core, &handle.data, handle.width, handle.height);

        atlas.tex_id = font_texture_id;
    }

    /// Creates and uploads a new wgpu texture made from the imgui font atlas.
    pub fn upload_texture(
        &mut self,
        core: &mut render::RenderCore,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> TextureId {

        // Create the wgpu texture.
        let texture = core.device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width,
                height,
                depth: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        });

        // Upload the actual data to the texture.
        core.queue.write_texture(
            TextureCopyView {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d { x: 0, y: 0, z: 0 },
            },
            data,
            TextureDataLayout {
                offset: 0,
                bytes_per_row: data.len() as u32 / height,
                rows_per_image: height,
            },
            Extent3d {
                width,
                height,
                depth: 1,
            },
        );
        
        let texture = ImguiTexture::new(texture, &self.texture_layout, core.device);
        self.textures.insert(texture)
    }
}



//
//
// Gui Rendering
//
//


pub struct ImguiStage<'r, W: gui::Widget> {
    pub window_state: &'r mut window::WindowState,
    pub imgui_renderer: &'r mut ImguiRenderer,
    pub widget: &'r mut W,
    pub render_target: &'r TextureView,
}

impl<W: gui::Widget> render::RenderStage for ImguiStage<'_, W> {
    fn encode(self, core: &mut render::RenderCore, encoder: &mut CommandEncoder) {
        
        let window::WindowState {
            ref mut imgui,
            ref mut platform,
            ref     window,
            ref     lua,
        } = self.window_state;
        
        platform.prepare_frame(imgui.io_mut(), window)
            .expect("Failed to prepare imgui frame.");

        let ui = imgui.frame();

        self.widget.compose(&ui, lua);

        platform.prepare_render(&ui, window);

        let draw_data = ui.render();
        self.imgui_renderer.render(draw_data, core, encoder, self.render_target);

    }
}
