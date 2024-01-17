use graphics::{ *};

pub const ACTION_SIZE: usize = 2;
pub const SCREEN_ZOOM: f32 = 2.0;

pub const BOARD_SIZE: f32 = 12.0;

pub const SHIP_ORDER: f32 = 3.2;
pub const ICON_ORDER: f32 = 3.1;
pub const EXPLOSION_ORDER: f32 = 3.0;
pub const GUI_BG_ORDER: f32 = 2.2;
pub const GUI_SHADE_ORDER: f32 = 2.1;
pub const GUI_RESULT_ORDER: f32 = 2.0;

pub struct TextureAllocation {
    pub ship_texture: Allocation,
    pub icon_texture: Allocation,
    pub explosion_texture: Allocation,
    pub game_bg_texture: Allocation,
    pub result_texture: Allocation,
    pub white_texture: Allocation,
}