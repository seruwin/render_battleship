use graphics::{ *};
use crate::EXPLOSION_ORDER;
use crate::TextureAllocation;

pub struct Animation {
    pub sprite: Image,
    pub in_play: bool,
    pub frame: i32,
    pub max_frame: i32,
    pub elapsed_time: f32,
}

impl Animation {
    pub fn new(resource: &TextureAllocation, renderer: &mut GpuRenderer) -> Self {
        let mut result = Self {
            sprite: Image::new(Some(resource.explosion_texture), renderer, 1),
            in_play: false,
            frame: -1,
            max_frame: 9,
            elapsed_time: 0.0,
        };
        result.sprite.pos = Vec3::new(0.0, 0.0, EXPLOSION_ORDER);
        result.sprite.hw = Vec2::new(60.0, 60.0);
        result.sprite.uv = Vec4::new(60.0 * 5.0, 0.0, 60.0, 60.0);
        result.sprite.color = Color::rgba(255, 255, 255, 255);
        result
    }

    pub fn update_frame(&mut self, seconds: f32) {
        if self.in_play {
            if self.elapsed_time + 0.07 <= seconds {
                self.elapsed_time = seconds;
                self.frame += 1;
                if self.frame >= self.max_frame {
                    self.in_play = false;
                    self.frame = -1;
                } else {
                    self.sprite.uv = Vec4::new(60.0 * (self.frame as f32), 0.0, 60.0, 60.0);
                    self.sprite.changed = true;
                }
            }
        }
    }

    pub fn play(&mut self, pos: Vec2) {
        if !self.in_play {
            self.sprite.pos = Vec3::new(pos.x, pos.y, EXPLOSION_ORDER);
            self.sprite.changed = true;
            self.in_play = true;
        }
    }
}
