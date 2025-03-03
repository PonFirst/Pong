pub struct Ball {
    pub x: f32,
    pub y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub radius: f32,
}

impl Ball {
    pub fn new(x: f32, y: f32, velocity_x: f32, velocity_y: f32, radius: f32) -> Self {
        Ball {
            x,
            y,
            velocity_x,
            velocity_y,
            radius,
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        self.x += self.velocity_x * delta_time;
        self.y += self.velocity_y * delta_time;
    }

    pub fn reset(&mut self, x: f32, y: f32, velocity_x: f32, velocity_y: f32) {
        self.x = x;
        self.y = y;
        self.velocity_x = velocity_x;
        self.velocity_y = velocity_y;
    }
}
