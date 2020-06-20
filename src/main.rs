
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};


pub mod render;

use render::RenderCore;
use render::window::WindowState;
use render::window::gui::GuiState;


fn main() -> ! {
    
    let event_loop = EventLoop::new();

    let mut window_state = WindowState::new(&event_loop);

    let mut renderer = futures::executor::block_on(RenderCore::init(&mut window_state));
    
    let mut gui = GuiState::new();
    let tri = render::TriangleRenderer::new(&mut renderer);

    let mut lua = rlua::Lua::new();
    
    // TODO: add ECS processing features
    let mut _world = hecs::World::new();

    let mut last_frame_time = std::time::Instant::now();

    // mainloop
    event_loop.run(move |event, _, control_flow| {

        window_state.handle_event(&event);
        
        match &event {
            Event::NewEvents(_) => {
                last_frame_time = window_state.update_frame_time(last_frame_time);
            },

            Event::WindowEvent { event, .. } => match *event {
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    window_state.update_scale_factor(scale_factor);
                },

                WindowEvent::Resized(new_size) => {
                    // WTF: Apparently, this actually causes a panic (!?)
                    // Fortunately it also appears to be unnecessary...
                    // window.set_inner_size(new_size); 
                    renderer.handle_window_resize(new_size);
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

                _ => (), //eprintln!("{:?}", event),
            },

            Event::MainEventsCleared => {
                window_state.window().request_redraw();
            },
            
            Event::RedrawRequested(_) => {

                let frame = match renderer.swap_chain.get_next_texture() {
                    Ok(frame) => frame,
                    Err(_) => {
                        eprintln!("Dropped frame!");
                        return;
                    }
                };

                renderer.sequence()
                    .draw(tri.with_target(&frame.view))
                    .draw(window_state.stage(&mut gui, &mut lua, &frame.view))
                    .finish();

            },

            _ => (),
        };

    });
}

