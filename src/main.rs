use std::{cell::RefCell, rc::Rc};

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

    let game_objects: Vec<Rc<RefCell<dyn GameObject>>> = vec![
        Rc::new(RefCell::new(left_paddle)),
        Rc::new(RefCell::new(ball)),
        Rc::new(RefCell::new(right_paddle))
    ];

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
            update(&game_objects);
            render_engine.draw(&game_objects);
        },
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { input, .. },
            ..
        } => {
            handle_keyboard_input(input, &game_objects[0]);
        }
        _ => ()
    });
}

fn update(game_objects: &Vec<Rc<RefCell<dyn GameObject>>>) {
    for game_object in game_objects {
        game_object.borrow_mut().update(game_objects);
    }
}

fn handle_keyboard_input(keyboard_input: KeyboardInput, game_object: &Rc<RefCell<dyn GameObject>>) {
    match keyboard_input {
        KeyboardInput {
            virtual_keycode: Some(VirtualKeyCode::Up),
            ..
        } => {
            game_object.borrow_mut().r#move(0, 2);
        },
        KeyboardInput {
            virtual_keycode: Some(VirtualKeyCode::Down),
            ..
        } => {
            game_object.borrow_mut().r#move(0, -2);
        },
        KeyboardInput {
            virtual_keycode: Some(VirtualKeyCode::Left),
            ..
        } => {
            game_object.borrow_mut().r#move(-2, 0);
        },
        KeyboardInput {
            virtual_keycode: Some(VirtualKeyCode::Right),
            ..
        } => {
            game_object.borrow_mut().r#move(2, 0);
        },
        _ => ()
    };
}
