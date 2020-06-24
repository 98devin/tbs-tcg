
use nalgebra_glm as glm;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent, DeviceEvent},
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

    let mut mouse_down = false;
    let mut modifiers = winit::event::ModifiersState::empty();

    // mainloop
    event_loop.run(move |event, _, control_flow| {

        window_state.platform.handle_event(
            window_state.imgui.io_mut(),
            &window_state.window,
            &event,
        );

        let imgui_wants_mouse = window_state.imgui.io().want_capture_mouse;
        let imgui_wants_kbord = window_state.imgui.io().want_capture_keyboard;
        
        match &event {
            Event::NewEvents(_) => {
                let (frame_time, frame_dura) = window_state.update_frame_time(last_frame_time);
                last_frame_time = frame_time;
                last_frame_duration = frame_dura;
            },

            Event::DeviceEvent { event, .. } => match *event {
                DeviceEvent::MouseMotion { delta } if mouse_down && !imgui_wants_mouse => {
                    if modifiers.shift() {
                        basic_renderer.camera.translate_rel(
                            if modifiers.ctrl() { 
                                glm::vec3(delta.0 as f32 / 100.0, 0.0, delta.1 as f32 / 100.0)
                            } else {
                                glm::vec3(delta.0 as f32 / 100.0, delta.1 as f32 / 100.0, 0.0)
                            }
                        );
                    }
                    else {
                        basic_renderer.camera.gimbal_lr(delta.0 as f32 / 100.0);
                        basic_renderer.camera.gimbal_ud(-delta.1 as f32 / 100.0);
                    }
                },

                _ => (),
            },

            Event::WindowEvent { event, .. } => match *event {

                WindowEvent::ModifiersChanged(new_modifiers) =>
                    modifiers = new_modifiers,

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

                WindowEvent::CloseRequested =>
                    *control_flow = ControlFlow::Exit,

                WindowEvent::MouseInput {
                    state, button: winit::event::MouseButton::Left, ..
                } if !imgui_wants_mouse => {
                    mouse_down = match state {
                        winit::event::ElementState::Pressed  => true,
                        winit::event::ElementState::Released => false,
                    };
                }

                WindowEvent::MouseWheel {
                    phase: winit::event::TouchPhase::Moved,
                    delta,
                    ..
                } if !imgui_wants_mouse => {
                    match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, ud) =>
                            basic_renderer.camera.zoom(ud / 25.0),
                        winit::event::MouseScrollDelta::PixelDelta(winit::dpi::LogicalPosition { y: ud, .. }) =>
                            basic_renderer.camera.zoom(ud as f32 / 100.0),
                    }
                }

                WindowEvent::KeyboardInput {
                    input: KeyboardInput {
                        virtual_keycode: Some(k),
                        state: ElementState::Pressed,
                        ..
                    },
                    ..
                } if !imgui_wants_kbord => {
                    let ratio = last_frame_duration.as_secs_f32();
                    match k {
                        VirtualKeyCode::Left =>
                            basic_renderer.camera.gimbal_lr(-10.0 * ratio),
                        VirtualKeyCode::Right =>
                            basic_renderer.camera.gimbal_lr(10.0 * ratio),
                        VirtualKeyCode::Up =>
                            basic_renderer.camera.gimbal_ud(-10.0 * ratio),
                        VirtualKeyCode::Down =>
                            basic_renderer.camera.gimbal_ud(10.0 * ratio),
                        VirtualKeyCode::Equals =>
                            basic_renderer.camera.zoom(0.5 * ratio),
                        VirtualKeyCode::Minus =>
                            basic_renderer.camera.zoom(-0.5 * ratio),

                        VirtualKeyCode::Escape =>
                            *control_flow = ControlFlow::Exit,

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


