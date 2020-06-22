

use nalgebra_glm as glm;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

pub mod render;

use render::RenderCore;
use render::window::WindowState;
use render::gui;


fn main() -> ! {
    
    let event_loop = EventLoop::new();

    let mut window_state = WindowState::new(&event_loop);
    
    let mut renderer = futures::executor::block_on(RenderCore::init(&mut window_state));
    
    let sc_format = renderer.sc_desc.format;

    let mut basic_renderer = render::BasicRenderer::new(&mut renderer);
    let mut imgui_renderer = gui::imgui_wgpu::ImguiRenderer::new(
        &mut renderer,
        sc_format,
        None,
    );

    imgui_renderer.build_font_texture(
        &mut renderer,
        &mut window_state.imgui
    );


    let mut gui = gui::GuiComponentState::new();

    // TODO: add ECS processing features
    let mut _world = hecs::World::new();

    let mut last_frame_time = std::time::Instant::now();
    let mut last_frame_duration = std::time::Duration::new(0, 0);

    // mainloop
    event_loop.run(move |event, _, control_flow| {

        window_state.platform.handle_event(
            window_state.imgui.io_mut(),
            &window_state.window,
            &event,
        );
        
        match &event {
            Event::NewEvents(_) => {
                let (frame_time, frame_dura) = window_state.update_frame_time(last_frame_time);
                last_frame_time = frame_time;
                last_frame_duration = frame_dura;
            },

            Event::WindowEvent { event, .. } => match *event {
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    window_state.update_scale_factor(scale_factor);
                    imgui_renderer.build_font_texture(&mut renderer, &mut window_state.imgui);
                },

                WindowEvent::Resized(new_size) => {
                    // WTF: Apparently, this actually causes a panic (!?)
                    // Fortunately it also appears to be unnecessary...
                    // window.set_inner_size(new_size); 
                    renderer.handle_window_resize(new_size);
                    basic_renderer.adjust_screen_res(new_size);
                    eprintln!("resized: {:?}", new_size);
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


                WindowEvent::KeyboardInput {
                    input: KeyboardInput {
                        virtual_keycode: Some(k),
                        state: ElementState::Pressed,
                        ..
                    },
                    ..
                } => {
                    match k {
                        VirtualKeyCode::Q =>
                            basic_renderer.camera.roll(5.0 * last_frame_duration.as_secs_f32()),
                        VirtualKeyCode::E =>
                            basic_renderer.camera.roll(-5.0 * last_frame_duration.as_secs_f32()),
                        VirtualKeyCode::W =>
                            basic_renderer.camera.translate_rel(
                                glm::vec3(0.0, 0.0, 5.0) * last_frame_duration.as_secs_f32()
                            ),
                        VirtualKeyCode::S =>
                            basic_renderer.camera.translate_rel(
                                glm::vec3(0.0, 0.0, -5.0) * last_frame_duration.as_secs_f32()
                            ),
                        VirtualKeyCode::A =>
                            basic_renderer.camera.translate_rel(
                                glm::vec3(-5.0, 0.0, 0.0) * last_frame_duration.as_secs_f32()
                            ),
                        VirtualKeyCode::D =>
                            basic_renderer.camera.translate_rel(
                                glm::vec3(5.0, 0.0, 0.0) * last_frame_duration.as_secs_f32()
                            ),
                        VirtualKeyCode::LShift =>
                            basic_renderer.camera.translate_rel(
                                glm::vec3(0.0, -5.0, 0.0) * last_frame_duration.as_secs_f32()
                            ),
                        VirtualKeyCode::Space =>
                            basic_renderer.camera.translate_rel(
                                glm::vec3(0.0, 5.0, 0.0) * last_frame_duration.as_secs_f32()
                            ),

                        VirtualKeyCode::Left =>
                            basic_renderer.camera.yaw(5.0 * last_frame_duration.as_secs_f32()),
                        VirtualKeyCode::Right =>
                            basic_renderer.camera.yaw(-5.0 * last_frame_duration.as_secs_f32()),
                        VirtualKeyCode::Up =>
                            basic_renderer.camera.pitch(-5.0 * last_frame_duration.as_secs_f32()),
                        VirtualKeyCode::Down =>
                            basic_renderer.camera.pitch(5.0 * last_frame_duration.as_secs_f32()),

                        _ => (),
                    }
                }

                _ => (), //eprintln!("{:?}", event),
            },

            Event::MainEventsCleared => {
                window_state.window.request_redraw();
            },
            
            Event::RedrawRequested(_) => {

                let frame = match renderer.swap_chain.get_next_frame() {
                    Ok(frame) => frame.output,
                    Err(_) => {
                        eprintln!("Dropped frame!");
                        return;
                    }
                };

                renderer.sequence()
                    .draw(render::BasicStage {
                        basic_renderer: &basic_renderer,
                        render_target: &frame.view,
                    })
                    .draw(gui::imgui_wgpu::ImguiStage {
                        window_state: &mut window_state,
                        imgui_renderer: &mut imgui_renderer,
                        widget: &mut gui,
                        render_target: &frame.view,
                    })
                    .finish();

            },

            _ => (),
        };

    });
}


