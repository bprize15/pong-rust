use crate::{MAX_POS, MIN_POS};

pub trait GameObject {
    fn r#move(&mut self, x_distance: i32, y_distance: i32) {
        let new_x = self.get_state().x as i32 + x_distance;
        if new_x < MIN_POS as i32 {
            self.get_state_mut().x = MIN_POS;
        } else if new_x + (self.get_state().width as i32) > MAX_POS as i32 {
            self.get_state_mut().x = MAX_POS - self.get_state().width;
        } else {
            self.get_state_mut().x = new_x as usize;
        }

        let new_y = self.get_state().y as i32 + y_distance;
        if new_y < MIN_POS as i32 {
            self.get_state_mut().y = MIN_POS;
        } else if new_y + (self.get_state().height as i32) > MAX_POS as i32 {
            self.get_state_mut().y = MAX_POS - self.get_state().height;
        } else {
            self.get_state_mut().y = new_y as usize;
        }
    }

    fn update(&mut self);

    fn get_state(&self) -> &GameObjectState;

    fn get_state_mut(&mut self) -> &mut GameObjectState;
}

pub struct Paddle {
    game_object_state: GameObjectState,
}

impl Paddle {
    pub fn new(game_object_state: GameObjectState) -> Self {
        Self { game_object_state }
    }
}

impl GameObject for Paddle {
    fn update(&mut self) {}

    fn get_state(&self) -> &GameObjectState {
        &self.game_object_state   
    }

    fn get_state_mut(&mut self) -> &mut GameObjectState {
        &mut self.game_object_state
    }
}

pub struct Ball {
    game_object_state: GameObjectState,
    velocity: i32
}

impl Ball {
    pub fn new(game_object_state: GameObjectState, velocity: i32) -> Self {
        Self { game_object_state, velocity }
    }

    pub fn update(&mut self) {
        self.r#move(self.velocity, 0);
    }
}

impl GameObject for Ball {
    fn update(&mut self) {
        self.r#move(0, self.velocity);
    }

    fn get_state(&self) -> &GameObjectState {
        &self.game_object_state
    }

    fn get_state_mut(&mut self) -> &mut GameObjectState {
        &mut self.game_object_state
    }
}

#[derive(Debug)]
pub struct GameObjectState {
    pub height: usize,
    pub width: usize,
    pub x: usize,
    pub y: usize,
}