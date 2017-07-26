


pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

pub fn vec2(x: u32, y: u32) -> Vec2 {
    Vec2 {
        x: x as f32,
        y: y as f32,
    }
}
