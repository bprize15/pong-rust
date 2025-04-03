use std::{cell::RefCell, rc::Rc, time::SystemTime};

use pong::{Ball, GameObject, GameObjectState, MoveCommand, Paddle, PaddleType, RenderEngine};
use winit::{event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent}, event_loop::{ControlFlow, EventLoop}};

fn main() {
    let event_loop = EventLoop::new();
    let mut render_engine = RenderEngine::new(&event_loop);

    let left_paddle = Paddle::new(
        GameObjectState { 
            height: 10.0, 
            width: 2.0,
            x: 0.0, 
            y: 50.0, 
        }, 
        PaddleType::PLAYER
    );
    let ball = Ball::new(
        GameObjectState { 
            height: 2.0, 
            width: 2.0,
            x: 50.0, 
            y: 50.0, 
        }, 
        1.0,
        0.0
    );
    let right_paddle = Paddle::new(
        GameObjectState { 
            height: 10.0, 
            width: 2.0,
            x: 98.0, 
            y: 50.0, 
        },
        PaddleType::AI
    );

    let game_objects: Vec<Rc<RefCell<dyn GameObject>>> = vec![
        Rc::new(RefCell::new(left_paddle)),
        Rc::new(RefCell::new(ball)),
        Rc::new(RefCell::new(right_paddle))
    ];

    let ms_per_update: u128 = 17;
    let mut previous = SystemTime::now();
    let mut lag: u128 = 0;

    event_loop.run(move |event, _, control_flow| {
        match event {
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
                let current = SystemTime::now();
                let elapsed = current
                    .duration_since(previous)
                    .expect("Current time is earlier than previous time");
                previous = current;
                lag += elapsed.as_millis();

                while lag >= ms_per_update {
                    update(&game_objects);
                    lag -= ms_per_update
                }
                render_engine.draw(&game_objects);
            },
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } => {
                let player_paddle = game_objects.iter().find(|game_object| {
                    if let Some(paddle) = game_object.borrow_mut().as_paddle() {
                        return paddle.paddle_type == PaddleType::PLAYER;
                    }
                    false
                }).
                expect("No player paddle found");
                handle_keyboard_input(input, player_paddle.borrow_mut().as_paddle().unwrap());
            }
            _ => ()
        }
    });
}

fn update(game_objects: &Vec<Rc<RefCell<dyn GameObject>>>) {
    for game_object in game_objects {
        game_object.borrow_mut().update(game_objects);
    }
}

fn handle_keyboard_input(keyboard_input: KeyboardInput, player_paddel: &mut Paddle) {
    match keyboard_input {
        KeyboardInput {
            virtual_keycode: Some(VirtualKeyCode::Up),
            ..
        } => {
            player_paddel.move_commands.push_back(MoveCommand::UP);
        },
        KeyboardInput {
            virtual_keycode: Some(VirtualKeyCode::Down),
            ..
        } => {
            player_paddel.move_commands.push_back(MoveCommand::DOWN);
        },
        _ => ()
    };
}
