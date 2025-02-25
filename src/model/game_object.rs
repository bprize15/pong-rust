use crate::RenderEngine;

pub struct GameObject {
    pub height: usize,
    pub width: usize,
    pub x: usize,
    pub y: usize
}

impl GameObject {
    pub fn new(height: usize , width: usize, x: usize, y: usize) -> Self {
        Self { height, width, x, y }
    }

    pub fn draw(&self, render_engine: RenderEngine) {
        render_engine.render(self);
    }

    pub fn move_vertically(&mut self, new_y: usize) {
        self.y = new_y;
    }
}