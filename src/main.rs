
use nalgebra as na;
use nalgebra_glm as glm;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent, DeviceEvent},
    event_loop::{ControlFlow, EventLoop},
};

pub mod render;
pub mod util;


use render::{Pass, AnyAttachmentDescriptor::*};
use render::window::WindowState;
use render::gui;


enum EngineEvent {
    RefreshRenderPasses {
        clear_asset_caches: bool,
    },
}



fn main() -> ! {
    
    let event_loop = EventLoop::<EngineEvent>::with_user_event();

    let mut window_state = WindowState::new(&event_loop);
    
    let mut renderer = futures::executor::block_on(render::Core::init(&mut window_state));
    
    let (mut main_pass, _) =
        render::MainPass::construct(1.0, (&renderer, &renderer.sc_desc));

    let (mut imgui_pass, _) =
        gui::imgui_wgpu::ImguiPass::construct(None, (
            &renderer,
            &mut window_state,
            SwapChain(&renderer.sc_desc),
        ));

    eprintln!("initial size: {:?}", window_state.window.inner_size());
    eprintln!("initial scale: {}", window_state.platform.hidpi_factor());

    let mut gui = gui::GuiComponentState::new();

    // TODO: add ECS processing features
    let mut _world = hecs::World::new();

    let mut last_frame_time = std::time::Instant::now();
    let mut last_frame_duration = std::time::Duration::new(0, 0);

    let mut mouse_down = false;
    let mut modifiers = winit::event::ModifiersState::empty();

    let mut debug_view: bool = true;

    let event_proxy = event_loop.create_proxy();

    // mainloop
    event_loop.run(move |event, _, control_flow| {

        window_state.platform.handle_event(
            window_state.imgui.io_mut(),
            &window_state.window,
            &event,
        );

        let imgui_wants_mouse = debug_view && window_state.imgui.io().want_capture_mouse;
        let imgui_wants_kbord = debug_view && window_state.imgui.io().want_capture_keyboard;
        
        match &event {
            Event::NewEvents(_) => {
                let (frame_time, frame_dura) = window_state.update_frame_time(last_frame_time);
                last_frame_time = frame_time;
                last_frame_duration = frame_dura;
            },

            Event::DeviceEvent { event, .. } => match *event {
                DeviceEvent::MouseMotion { delta } if mouse_down && !imgui_wants_mouse => {
                    if modifiers.shift() {
                        main_pass.basic.camera.translate_rel(
                            if modifiers.ctrl() { 
                                glm::vec3(delta.0 as f32 / 100.0, 0.0, delta.1 as f32 / 100.0)
                            } else {
                                glm::vec3(delta.0 as f32 / 100.0, delta.1 as f32 / 100.0, 0.0)
                            }
                        );
                    }
                    else {
                        main_pass.basic.camera.gimbal_lr(delta.0 as f32 / 100.0);
                        main_pass.basic.camera.gimbal_ud(-delta.1 as f32 / 100.0);
                    }
                },

                _ => (),
            },

            Event::UserEvent(e_event) => match *e_event {
                EngineEvent::RefreshRenderPasses { clear_asset_caches } => {
                    
                    eprintln!("Refreshing render pipelines...");

                    if clear_asset_caches {
                        eprintln!("Resetting asset caches...");
                        use render::cache::AssetCache;
                        renderer.shaders.clear();
                        renderer.textures.clear();
                        renderer.models.clear();
                    }

                    let _ = main_pass.refresh(1.0, (
                        &renderer,
                        &renderer.sc_desc,
                    ));

                    let _ = imgui_pass.refresh(None, (
                        &renderer,
                        &mut window_state,
                        SwapChain(&renderer.sc_desc),
                    ));
                }
            }

            Event::WindowEvent { event, .. } => match *event {

                WindowEvent::ModifiersChanged(new_modifiers) =>
                    modifiers = new_modifiers,

                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    window_state.update_scale_factor(scale_factor);
                    imgui_pass.build_font_texture(&mut renderer, &mut window_state.imgui);
                    eprintln!("updated scale factor: {}", scale_factor);
                },

                WindowEvent::Resized(new_size) => {
                    // WTF: Apparently, this actually causes a panic (!?)
                    // Fortunately it also appears to be unnecessary...
                    // window.set_inner_size(new_size); 
                    renderer.handle_window_resize(new_size);
                    eprintln!("resized: {:?}", new_size);
                    event_proxy.send_event(
                        EngineEvent::RefreshRenderPasses {
                            clear_asset_caches: false,
                        }
                    ).ok().unwrap();
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
                            main_pass.basic.camera.zoom(ud / 25.0),
                        winit::event::MouseScrollDelta::PixelDelta(winit::dpi::LogicalPosition { y: ud, .. }) =>
                        main_pass.basic.camera.zoom(ud as f32 / 100.0),
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
                            main_pass.basic.camera.gimbal_lr(-10.0 * ratio),
                        VirtualKeyCode::Right =>
                            main_pass.basic.camera.gimbal_lr(10.0 * ratio),
                        VirtualKeyCode::Up =>
                            main_pass.basic.camera.gimbal_ud(-10.0 * ratio),
                        VirtualKeyCode::Down =>
                            main_pass.basic.camera.gimbal_ud(10.0 * ratio),
                        VirtualKeyCode::Equals =>
                            main_pass.basic.camera.zoom(0.5 * ratio),
                        VirtualKeyCode::Minus =>
                            main_pass.basic.camera.zoom(-0.5 * ratio),

                        VirtualKeyCode::Escape =>
                            *control_flow = ControlFlow::Exit,

                        VirtualKeyCode::Grave =>
                            debug_view = !debug_view,

                        VirtualKeyCode::R if modifiers.ctrl() =>
                            event_proxy.send_event(
                                EngineEvent::RefreshRenderPasses {
                                    clear_asset_caches: true,
                                }
                            ).ok().unwrap(),

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
                    Ok(frame) => frame,
                    Err(_) => {
                        eprintln!("Dropped frame!");
                        return;
                    }
                };

                let _ = main_pass.perform((), (
                    &renderer,
                    &frame.output.view,
                ));

                if debug_view {
                    let _ = imgui_pass.perform(&mut gui, (
                        &renderer,
                        &mut window_state,
                        &frame.output.view,
                    ));
                }

            },

            _ => (),
        };

    });
}


