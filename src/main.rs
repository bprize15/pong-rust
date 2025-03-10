use pong::{GameObject, RenderEngine};
use winit::{event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent}, event_loop::{ControlFlow, EventLoop}};

// make static list of game objects? Then transform the list

fn main() {
    let event_loop = EventLoop::new();
    let mut render_engine = RenderEngine::new(&event_loop);

    let left_paddle = GameObject::new(0.5, 0.05, -0.9, 0.0);
    let ball = GameObject::new(0.05, 0.05, 0.0, 0.0);
    let right_paddle = GameObject::new(0.5, 0.05, 0.9, 0.0);

    let mut game_objects = vec![left_paddle, ball, right_paddle];

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { 
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        },
        Event::WindowEvent { 
            event: WindowEvent::Resized(_),
            ..
        } => {
            render_engine.on_window_resized();
        }
        Event::MainEventsCleared => { // Main game loop
            // process input
            // update
            render_engine.render(&game_objects);
        },
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { input, .. },
            ..
        } => {
            handle_keyboard_input(input, &mut game_objects[0]);
        }
        _ => ()
    });
}

fn handle_keyboard_input(keyboard_input: KeyboardInput, game_object: &mut GameObject) {
    match keyboard_input {
        KeyboardInput {
            virtual_keycode: Some(VirtualKeyCode::Up),
            ..
        } => {
            game_object.move_vertically(0.1);
        },
        KeyboardInput {
            virtual_keycode: Some(VirtualKeyCode::Down),
            ..
        } => {
            game_object.move_vertically(-0.1);
        },
        _ => ()
    };
}
