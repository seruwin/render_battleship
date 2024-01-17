use glam::f32::*;
use winit::dpi::PhysicalSize;

use crate::{
    BOARD_SIZE,
    SCREEN_ZOOM,
};

pub fn get_tile_pos(x: i32, y: i32) -> usize {
    (x + (y * BOARD_SIZE as i32)) as usize
}

pub fn find_x_base_on_tile(tile: usize) -> usize {
    tile % BOARD_SIZE as usize
}

pub fn find_y_base_on_tile(tile: usize) -> usize {
    tile / BOARD_SIZE as usize
}

pub fn world_to_sprite_3pos(pos: &Vec3, size: &PhysicalSize<f32>) -> Vec3 {
    Vec3::new(pos.x / SCREEN_ZOOM, (size.height - pos.y) / SCREEN_ZOOM, pos.z)
}

pub fn world_to_sprite_2pos(pos: &Vec2, size: &PhysicalSize<f32>) -> Vec2 {
    Vec2::new(pos.x / SCREEN_ZOOM, (size.height - pos.y) / SCREEN_ZOOM)
}

pub fn tile_to_render_pos(pos: &Vec2, map_start_pos: Vec2, tile_size: f32) -> Vec2 {
    Vec2::new(map_start_pos.x + (pos.x * tile_size), map_start_pos.y + (pos.y * tile_size))
}