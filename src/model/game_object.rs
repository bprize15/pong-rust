pub struct GameObject {
    pub height: f32,
    pub width: f32,
    pub x: f32,
    pub y: f32
}

impl GameObject {
    pub fn new(height: f32 , width: f32, x: f32, y: f32) -> Self {
        Self { height, width, x, y }
    }

    pub fn move_vertically(&mut self, y_distance: f32) {
        self.y += -1.0 * y_distance;
    }

    pub fn move_horizontally(&mut self, x_distance: f32) {
        self.x += x_distance;
    }

    pub fn r#move(&mut self, x_distance: f32, y_distance: f32) {
        self.move_horizontally(x_distance);
        self.move_vertically(y_distance);
    }
}