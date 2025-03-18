use pong::{Ball, GameObject, GameObjectState, Paddle, RenderEngine};
use winit::{event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent}, event_loop::{ControlFlow, EventLoop}};

fn main() {
    let event_loop = EventLoop::new();
    let mut render_engine = RenderEngine::new(&event_loop);

    let left_paddle = Paddle::new(
        GameObjectState { 
            height: 10, 
            width: 2,
            x: 1, 
            y: 50, 
        }, 
    );
    let ball = Ball::new(
        GameObjectState { 
            height: 2, 
            width: 2,
            x: 50, 
            y: 50, 
        }, 
        1
    );
    let right_paddle = Paddle::new(
        GameObjectState { 
            height: 10, 
            width: 2,
            x: 99, 
            y: 50, 
        },
    );

    let mut game_objects: Vec<Box<dyn GameObject>> = vec![Box::new(left_paddle), Box::new(ball), Box::new(right_paddle)];

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
            render_engine.draw(&game_objects);
            update(&mut game_objects);
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

fn update(game_objects: &mut Vec<Box<dyn GameObject>>) {
    game_objects.iter_mut().for_each(|game_object| game_object.update());
}

fn handle_keyboard_input(keyboard_input: KeyboardInput, game_object: &mut Box<dyn GameObject>) {
    match keyboard_input {
        KeyboardInput {
            virtual_keycode: Some(VirtualKeyCode::Up),
            ..
        } => {
            game_object.r#move(0, 2);
        },
        KeyboardInput {
            virtual_keycode: Some(VirtualKeyCode::Down),
            ..
        } => {
            game_object.r#move(0, -2);
        },
        KeyboardInput {
            virtual_keycode: Some(VirtualKeyCode::Left),
            ..
        } => {
            game_object.r#move(-2, 0);
        },
        KeyboardInput {
            virtual_keycode: Some(VirtualKeyCode::Right),
            ..
        } => {
            game_object.r#move(2, 0);
        },
        _ => ()
    };
}
