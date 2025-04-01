use std::{cell::RefCell, collections::VecDeque, ptr, rc::Rc};

use crate::{MAX_POS, MIN_POS};

pub trait GameObject {
    fn r#move(&mut self, x_distance: f32, y_distance: f32) {
        let new_x = self.get_state().x + x_distance;
        if new_x < MIN_POS {
            self.get_state_mut().x = MIN_POS;
        } else if new_x + self.get_state().width > MAX_POS {
            self.get_state_mut().x = MAX_POS - self.get_state().width;
        } else {
            self.get_state_mut().x = new_x;
        }

        let new_y = self.get_state().y + y_distance;
        if new_y < MIN_POS {
            self.get_state_mut().y = MIN_POS;
        } else if new_y + self.get_state().height > MAX_POS {
            self.get_state_mut().y = MAX_POS - self.get_state().height;
        } else {
            self.get_state_mut().y = new_y;
        }
    }

    fn update(&mut self, game_objects: &Vec<Rc<RefCell<dyn GameObject>>>);

    fn get_state(&self) -> &GameObjectState;

    fn get_state_mut(&mut self) -> &mut GameObjectState;

    fn as_paddle(&mut self) -> Option<&mut Paddle>;
}

pub struct Paddle {
    game_object_state: GameObjectState,
    pub paddle_type: PaddleType,
    pub move_commands: VecDeque<MoveCommand>
}

impl Paddle {
    pub fn new(game_object_state: GameObjectState, paddle_type: PaddleType) -> Self {
        Self { game_object_state, paddle_type, move_commands: VecDeque::new() }
    }
}

impl GameObject for Paddle {
    fn update(&mut self, _game_objects: &Vec<Rc<RefCell<dyn GameObject>>>) {
        while let Some(move_command) = self.move_commands.pop_front() {
            match move_command {
                MoveCommand::UP => self.r#move(0.0, 4.0),
                MoveCommand::DOWN => self.r#move(0.0, -4.0),
            }
        }
    }

    fn get_state(&self) -> &GameObjectState {
        &self.game_object_state   
    }

    fn get_state_mut(&mut self) -> &mut GameObjectState {
        &mut self.game_object_state
    }

    fn as_paddle(&mut self) -> Option<&mut Self> {
        Some(self)
    }
}

pub struct Ball {
    game_object_state: GameObjectState,
    velocity_x: f32,
    velocity_y: f32
}

impl Ball {
    pub fn new(game_object_state: GameObjectState, velocity_x: f32, velocity_y: f32) -> Self {
        Self { game_object_state, velocity_x, velocity_y }
    }
}

impl GameObject for Ball {
    fn update(&mut self, game_objects: &Vec<Rc<RefCell<dyn GameObject>>>) {
        for game_object in game_objects {
            if ptr::addr_eq(self as &dyn GameObject, game_object.as_ptr()) {
                continue;
            }

            if self.get_state().x < game_object.borrow().get_state().x + game_object.borrow().get_state().width &&
                self.get_state().x + self.get_state().width > game_object.borrow().get_state().x &&
                self.get_state().y < game_object.borrow().get_state().y + game_object.borrow().get_state().height &&
                self.get_state().y + self.get_state().height > game_object.borrow().get_state().y
            {
                self.velocity_y = linear_interpolate(
                    self.get_state().y,
                    (game_object.borrow().get_state().y, game_object.borrow().get_state().y + game_object.borrow().get_state().height)
                );
                self.velocity_x *= -1.0;
            }
        }

        if self.get_state().y <= MIN_POS || self.get_state().y + self.get_state().height >= MAX_POS {
            self.velocity_y *= -1.0;
        }

        self.r#move(self.velocity_x, self.velocity_y);
    }

    fn get_state(&self) -> &GameObjectState {
        &self.game_object_state
    }

    fn get_state_mut(&mut self) -> &mut GameObjectState {
        &mut self.game_object_state
    }

    fn as_paddle(&mut self) -> Option<&mut Paddle> {
        None
    }
}

fn linear_interpolate(val: f32, source_range: (f32, f32)) -> f32 {
    let target_range = (-1.0, 1.0);
    target_range.0 + (val - source_range.0) * (target_range.1 - target_range.0) / (source_range.1 - source_range.0)
}

#[derive(Debug)]
pub struct GameObjectState {
    pub height: f32,
    pub width: f32,
    pub x: f32,
    pub y: f32,
}

pub enum MoveCommand {
    UP,
    DOWN
}

#[derive(PartialEq)]
pub enum PaddleType {
    PLAYER,
    AI
}