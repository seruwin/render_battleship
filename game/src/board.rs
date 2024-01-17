mod anim_icon;

pub use anim_icon::*;

use graphics::{ *};
use rand::Rng;
use cosmic_text::{Attrs, Metrics};
use std::collections::HashSet;
use winit::dpi::PhysicalSize;
use crate::func::*;
use crate::TextureAllocation;
use crate::{
    GUI_RESULT_ORDER,
    GUI_SHADE_ORDER,
    BOARD_SIZE,
    SHIP_ORDER,
    ICON_ORDER,
    SCREEN_ZOOM,
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BoardType {
    None,
    Ship(i32),
    Hit(i32),
    Missed,
}

pub struct Ship {
    pub sprite: Image,
    pub parts: i32,
    pub index: i32,
    pub visible: bool,
}

impl Ship {
    pub fn new(resource: &TextureAllocation, renderer: &mut GpuRenderer, index: i32, parts: i32) -> Self {
        Self {
            sprite: Image::new(Some(resource.ship_texture), renderer, 1),
            parts,
            index,
            visible: false,
        }
    }

    pub fn damage_ship(&mut self) -> bool {
        self.parts -= 1;
        if self.parts <= 0 {
            self.visible = true;
            true
        } else {
            false
        }
    }
}

pub struct GameBoard {
    pub got_winner: bool,
    pub current_turn: i32,
    pub size_count: [i32; 4],
    pub win_image: Image,
    pub lose_image: Image,
    pub board_shade: [Image; 2],
    pub status_text: Text,
    pub ship_counter: [[Text; 4]; 2],
    pub ship_counter_data: [[i32; 4]; 2],
}

impl GameBoard {
    pub fn new(resource: &TextureAllocation, renderer: &mut GpuRenderer, scale: &f64) -> Self {
        let mut rng = rand::thread_rng();
        let mut result = Self {
            got_winner: false,
            current_turn: 0,
            size_count: [
                rng.gen_range(3..=5),
                rng.gen_range(2..=4),
                rng.gen_range(1..=3),
                rng.gen_range(1..=2),
            ],
            win_image: Image::new(Some(resource.result_texture), renderer, 1),
            lose_image: Image::new(Some(resource.result_texture), renderer, 1),
            board_shade: [
                Image::new(Some(resource.white_texture), renderer, 1),
                Image::new(Some(resource.white_texture), renderer, 1),
            ],
            status_text: Text::new(renderer,
                            Some(Metrics::new(16.0, 16.0).scale(*scale as f32)),
                            Vec3::new(490.0, 4.0, 0.0),
                            Vec2::new(200.0, 16.0)),
            ship_counter: [
                [
                    Text::new(renderer,Some(Metrics::new(16.0, 16.0).scale(*scale as f32)), 
                                        Vec3::new(46.0 * SCREEN_ZOOM, 326.0 * SCREEN_ZOOM, 0.0), Vec2::new(32.0, 16.0)),
                    Text::new(renderer,Some(Metrics::new(16.0, 16.0).scale(*scale as f32)), 
                                        Vec3::new(46.0 * SCREEN_ZOOM, 301.0 * SCREEN_ZOOM, 0.0), Vec2::new(32.0, 16.0)),
                    Text::new(renderer,Some(Metrics::new(16.0, 16.0).scale(*scale as f32)), 
                                        Vec3::new(156.0 * SCREEN_ZOOM, 326.0 * SCREEN_ZOOM, 0.0), Vec2::new(32.0, 16.0)),
                    Text::new(renderer,Some(Metrics::new(16.0, 16.0).scale(*scale as f32)), 
                                        Vec3::new(156.0 * SCREEN_ZOOM, 301.0 * SCREEN_ZOOM, 0.0), Vec2::new(32.0, 16.0)),                        
                ],
                [
                    Text::new(renderer,Some(Metrics::new(16.0, 16.0).scale(*scale as f32)), 
                                        Vec3::new(315.0 * SCREEN_ZOOM, 326.0 * SCREEN_ZOOM, 0.0), Vec2::new(32.0, 16.0)),
                    Text::new(renderer,Some(Metrics::new(16.0, 16.0).scale(*scale as f32)), 
                                        Vec3::new(315.0 * SCREEN_ZOOM, 301.0 * SCREEN_ZOOM, 0.0), Vec2::new(32.0, 16.0)),
                    Text::new(renderer,Some(Metrics::new(16.0, 16.0).scale(*scale as f32)), 
                                        Vec3::new(425.0 * SCREEN_ZOOM, 326.0 * SCREEN_ZOOM, 0.0), Vec2::new(32.0, 16.0)),
                    Text::new(renderer,Some(Metrics::new(16.0, 16.0).scale(*scale as f32)), 
                                        Vec3::new(425.0 * SCREEN_ZOOM, 301.0 * SCREEN_ZOOM, 0.0), Vec2::new(32.0, 16.0)),                        
                ],
            ],
            ship_counter_data: [[0,0,0,0],[0,0,0,0]],
        };

        result.ship_counter_data = [
            result.size_count.clone(),
            result.size_count.clone(),
        ];

        result.status_text.set_buffer_size(renderer, renderer.size().width as i32, renderer.size().height as i32)
            .set_bounds(Some(Bounds::new(348.0, 0.0, 746.0, 20.0)))
            .set_default_color(Color::rgba(185, 185, 185, 255));

        result.ship_counter[0][0].set_buffer_size(renderer, renderer.size().width as i32, renderer.size().height as i32)
            .set_bounds(Some(Bounds::new(34.0 * SCREEN_ZOOM, 326.0 * SCREEN_ZOOM, 61.0 * SCREEN_ZOOM, 341.0 * SCREEN_ZOOM)))
            .set_default_color(Color::rgba(185, 185, 185, 255));
        result.ship_counter[0][1].set_buffer_size(renderer, renderer.size().width as i32, renderer.size().height as i32)
            .set_bounds(Some(Bounds::new(34.0 * SCREEN_ZOOM, 301.0 * SCREEN_ZOOM, 61.0 * SCREEN_ZOOM, 316.0 * SCREEN_ZOOM)))
            .set_default_color(Color::rgba(185, 185, 185, 255));
        result.ship_counter[0][2].set_buffer_size(renderer, renderer.size().width as i32, renderer.size().height as i32)
            .set_bounds(Some(Bounds::new(144.0 * SCREEN_ZOOM, 326.0 * SCREEN_ZOOM, 171.0 * SCREEN_ZOOM, 341.0 * SCREEN_ZOOM)))
            .set_default_color(Color::rgba(185, 185, 185, 255));
        result.ship_counter[0][3].set_buffer_size(renderer, renderer.size().width as i32, renderer.size().height as i32)
            .set_bounds(Some(Bounds::new(144.0 * SCREEN_ZOOM, 301.0 * SCREEN_ZOOM, 171.0 * SCREEN_ZOOM, 316.0 * SCREEN_ZOOM)))
            .set_default_color(Color::rgba(185, 185, 185, 255));

        result.ship_counter[1][0].set_buffer_size(renderer, renderer.size().width as i32, renderer.size().height as i32)
            .set_bounds(Some(Bounds::new(303.0 * SCREEN_ZOOM, 326.0 * SCREEN_ZOOM, 330.0 * SCREEN_ZOOM, 341.0 * SCREEN_ZOOM)))
            .set_default_color(Color::rgba(185, 185, 185, 255));
        result.ship_counter[1][1].set_buffer_size(renderer, renderer.size().width as i32, renderer.size().height as i32)
            .set_bounds(Some(Bounds::new(303.0 * SCREEN_ZOOM, 301.0 * SCREEN_ZOOM, 330.0 * SCREEN_ZOOM, 316.0 * SCREEN_ZOOM)))
            .set_default_color(Color::rgba(185, 185, 185, 255));
        result.ship_counter[1][2].set_buffer_size(renderer, renderer.size().width as i32, renderer.size().height as i32)
            .set_bounds(Some(Bounds::new(413.0 * SCREEN_ZOOM, 326.0 * SCREEN_ZOOM, 440.0 * SCREEN_ZOOM, 341.0 * SCREEN_ZOOM)))
            .set_default_color(Color::rgba(185, 185, 185, 255));
        result.ship_counter[1][3].set_buffer_size(renderer, renderer.size().width as i32, renderer.size().height as i32)
            .set_bounds(Some(Bounds::new(413.0 * SCREEN_ZOOM, 301.0 * SCREEN_ZOOM, 440.0 * SCREEN_ZOOM, 316.0 * SCREEN_ZOOM)))
            .set_default_color(Color::rgba(185, 185, 185, 255));

        for x in 0..=1 {
            for y in 0..=3 {
                result.ship_counter[x][y].set_text(renderer, "0", Attrs::new());
            }
        }

        result.status_text.set_text(renderer, "PLAYER 1 TURN", Attrs::new());

        result.win_image.pos = Vec3::new(0.0, 0.0, GUI_RESULT_ORDER);
        result.win_image.hw = Vec2::new(240.0, 44.0);
        result.win_image.uv = Vec4::new(0.0, 0.0, 240.0, 44.0);
        result.win_image.color = Color::rgba(255, 255, 255, 255);
        result.lose_image.pos = Vec3::new(0.0, 0.0, GUI_RESULT_ORDER);
        result.lose_image.hw = Vec2::new(240.0, 44.0);
        result.lose_image.uv = Vec4::new(0.0, 44.0, 240.0, 44.0);
        result.lose_image.color = Color::rgba(255, 255, 255, 255);

        result.board_shade[0].pos = Vec3::new(27.0, 11.0, GUI_SHADE_ORDER);
        result.board_shade[1].pos = Vec3::new(297.0, 11.0, GUI_SHADE_ORDER);
        for i in 0..=1 {
            result.board_shade[i].hw = Vec2::new(240.0, 240.0);
            result.board_shade[i].uv = Vec4::new(0.0, 0.0, 16.0, 16.0);
            result.board_shade[i].color = Color::rgba(0, 0, 0, 150);
        }

        result
    }

    pub fn set_winner(&mut self, index: i32) {
        if index > 0 {
            self.win_image.pos = Vec3::new(296.0, 110.0, GUI_RESULT_ORDER);
            self.lose_image.pos = Vec3::new(27.0, 110.0, GUI_RESULT_ORDER);
        } else {
            self.win_image.pos = Vec3::new(27.0, 110.0, GUI_RESULT_ORDER);
            self.lose_image.pos = Vec3::new(296.0, 110.0, GUI_RESULT_ORDER);
        }
        self.win_image.changed = true;
        self.lose_image.changed = true;
        self.got_winner = true;
    }

    pub fn change_status_text(&mut self, message: &str, renderer: &mut GpuRenderer) {
        self.status_text.set_text(
            renderer,
            message,
            Attrs::new(),
        );
    }

    pub fn update_ship_counter(&mut self, ship_count: &[i32; 4], renderer: &mut GpuRenderer, board_index: usize) {
        for y in 0..=3 {
            self.ship_counter[board_index][y].set_text(renderer, &format!("{}", ship_count[y]), Attrs::new());
        }
    }

    pub fn reduce_ship_counter(&mut self, ship_size: usize, renderer: &mut GpuRenderer, board_index: usize) {
        self.ship_counter_data[board_index][ship_size] -= 1;
        self.ship_counter[board_index][ship_size].set_text(renderer, &format!("{}", self.ship_counter_data[board_index][ship_size]), Attrs::new());
    }
}

pub struct Board {
    pub data: [BoardType; 256],
    pub ship: Vec<Ship>,
    pub icon: Vec<Image>,
    pub map: Map,
}

impl Board {
    pub fn new(renderer: &mut GpuRenderer, pos: Vec2) -> Self {
        let mut data = Self {
            data: [BoardType::None; 256],
            ship: Vec::with_capacity(1),
            icon: Vec::with_capacity(1),
            map: Map::new(renderer, 20),
        };
        (0..BOARD_SIZE as u32).for_each(|x| {
            (0..BOARD_SIZE as u32).for_each(|y| {
                data.map.set_tile((x, y, 2),TileData {texture_id: 1,texture_layer: 0,color: Color::rgba(255, 255, 255, 255)});
                data.map.set_tile((x, y, 3),TileData {texture_id: 2,texture_layer: 0,color: Color::rgba(255, 255, 255, 255)});
                data.map.set_tile((x, y, 4),TileData {texture_id: 3,texture_layer: 0,color: Color::rgba(255, 255, 255, 255)});
                data.map.set_tile((x, y, 5),TileData {texture_id: 4,texture_layer: 0,color: Color::rgba(255, 255, 255, 255)});
            });
        });

        data.map.pos = pos;
        data.map.can_render = true;
        data
    }

    pub fn count_ship(&mut self) -> i32 {
        let mut ship_index: Vec<i32>;
        ship_index = Vec::with_capacity(1);
        for data in self.data {
            if let BoardType::Ship(index) = data { ship_index.push(index); }
        }
        let set_data: HashSet<_> = ship_index.into_iter().collect();
        set_data.len() as i32
    }

    pub fn check_vertical(&mut self, pos: &Vec2, size: usize) -> bool {
        let mut result = true;
        for y in 0..=size {
            if pos.x >= 0.0 && pos.x < BOARD_SIZE && (pos.y + y as f32) >= 0.0 && (pos.y + y as f32) < BOARD_SIZE {
                let tile = get_tile_pos(pos.x as i32, (pos.y + y as f32) as i32);
                if self.data[tile] != BoardType::None {
                    result = false;
                    break;
                }
            } else {
                result = false;
                break;
            }
        }
        result
    }
    
    pub fn check_horizontal(&mut self, pos: &Vec2, size: usize) -> bool {
        let mut result = true;
        for x in 0..=size {
            if (pos.x + x as f32) >= 0.0 && (pos.x + x as f32) < BOARD_SIZE && pos.y >= 0.0 && pos.y < BOARD_SIZE {
                let tile = get_tile_pos((pos.x + x as f32) as i32, pos.y as i32);
                if self.data[tile] != BoardType::None {
                    result = false;
                    break;
                }
            } else {
                result = false;
                break;
            }
        }
        result
    }
    
    pub fn calculate_available_tile(&mut self, size: usize) -> Vec<i32> {
        let mut available_space: Vec<i32> = vec![];
    
        for i in 0..=(self.data.len() - 1) {
            if self.data[i] == BoardType::None {
                let tile_pos: Vec2 = Vec2::new(find_x_base_on_tile(i) as f32, find_y_base_on_tile(i) as f32);
                let mut add_block: bool;
    
                add_block = self.check_horizontal(&tile_pos, size);
                if !add_block {add_block = self.check_vertical(&tile_pos, size)}
    
                if add_block {
                    available_space.push(i as i32);
                }
            }
        }
        available_space
    }
    
    pub fn try_place_ship(&mut self, index: i32, size: usize) -> Option<(i32, Vec2)> {
        let available_space: Vec<i32> = self.calculate_available_tile(size);
        if available_space.is_empty() { return None; } 
    
        let mut rng = rand::thread_rng();
        let randomize_slot = rng.gen_range(1..=(available_space.len() - 1));
        let tile_index = available_space[randomize_slot] as usize;
        let random_dir = rng.gen_range(0..=1);
        let tile_pos: Vec2 = Vec2::new(find_x_base_on_tile(tile_index) as f32, find_y_base_on_tile(tile_index) as f32);
        let mut result: Option<(i32, Vec2)> = None;
        if random_dir == 1 {
            if self.check_vertical(&tile_pos, size) {
                for s in 0..=size {
                    let pos = Vec2::new(find_x_base_on_tile(tile_index) as f32, (find_y_base_on_tile(tile_index) + s) as f32);
                    let tilepos = get_tile_pos(pos.x as i32, pos.y as i32);
                    self.data[tilepos] = BoardType::Ship(index);
                }
                result = Some((0, tile_pos));
            } else if self.check_horizontal(&tile_pos, size) {
                for s in 0..=size {
                    let pos = Vec2::new((find_x_base_on_tile(tile_index) + s) as f32, find_y_base_on_tile(tile_index) as f32);
                    let tilepos = get_tile_pos(pos.x as i32, pos.y as i32);
                    self.data[tilepos] = BoardType::Ship(index);
                }
                result = Some((1, tile_pos));
            }
        } else {
            if self.check_horizontal(&tile_pos, size) {
                for s in 0..=size {
                    let pos = Vec2::new((find_x_base_on_tile(tile_index) + s) as f32, find_y_base_on_tile(tile_index) as f32);
                    let tilepos = get_tile_pos(pos.x as i32, pos.y as i32);
                    self.data[tilepos] = BoardType::Ship(index);
                }
                result = Some((1, tile_pos));
            } else if self.check_vertical(&tile_pos, size) {
                for s in 0..=size {
                    let pos = Vec2::new(find_x_base_on_tile(tile_index) as f32, (find_y_base_on_tile(tile_index) + s) as f32);
                    let tilepos = get_tile_pos(pos.x as i32, pos.y as i32);
                    self.data[tilepos] = BoardType::Ship(index);
                }
                result = Some((0, tile_pos));
            }
        }
        result
    }
    
    pub fn place_ship(&mut self, size: usize, index: i32, resource: &TextureAllocation, renderer: &mut GpuRenderer) -> bool {
        let result = self.try_place_ship(index, size);
        if result.is_none() { return false; }
    
        let mut rng = rand::thread_rng();
        let result_data = result.unwrap();
        let sprite_pos = tile_to_render_pos(&result_data.1, self.map.pos, 20.0);
        let mut ship = Ship::new(resource, renderer, index, (size + 1) as i32);
    
        if size >= 1 {
            let random_number = rng.gen_range(0..=1);
            if result_data.0 == 0 { // Vertical
                ship.sprite.pos = Vec3::new(sprite_pos.x, sprite_pos.y, SHIP_ORDER);
                ship.sprite.hw = Vec2::new(20.0, 20.0 + (20.0 * size as f32));
                match size {
                    1 => { ship.sprite.uv = Vec4::new(40.0 + (20.0 * (random_number as f32)), 20.0, 20.0, 40.0); },
                    2 => { ship.sprite.uv = Vec4::new(60.0 + (20.0 * (random_number as f32)), 60.0, 20.0, 60.0); },
                    _ => { ship.sprite.uv = Vec4::new(100.0, 0.0 + (80.0 * (random_number as f32)), 20.0, 80.0); },
                }
            } else { // Horizontal
                ship.sprite.pos = Vec3::new(sprite_pos.x, sprite_pos.y, SHIP_ORDER);
                ship.sprite.hw = Vec2::new(20.0 + (20.0 * size as f32), 20.0);
                match size {
                    1 => { ship.sprite.uv = Vec4::new(0.0, 20.0 + (20.0 * (random_number as f32)), 40.0, 20.0); },
                    2 => { ship.sprite.uv = Vec4::new(0.0, 60.0 + (20.0 * (random_number as f32)), 60.0, 20.0); },
                    _ => { ship.sprite.uv = Vec4::new(0.0, 120.0 + (20.0 * (random_number as f32)), 80.0, 20.0); },
                }
            }
        } else {
            let random_number = rng.gen_range(0..=3);
            ship.sprite.pos = Vec3::new(sprite_pos.x, sprite_pos.y, SHIP_ORDER);
            ship.sprite.hw = Vec2::new(20.0, 20.0);
            ship.sprite.uv = Vec4::new(20.0 * random_number as f32, 0.0, 20.0, 20.0);
        }
        ship.sprite.color = Color::rgba(255, 255, 255, 255);
        self.ship.push(ship);
    
        true
    }

    pub fn prepare_board(&mut self, resource: &TextureAllocation, renderer: &mut GpuRenderer, gameboard: &mut GameBoard, board_index: usize) {
        let mut cur_index = 0;
        for x in 0..=3 {
            for _i in 0..=gameboard.size_count[x] - 1 {
                if self.place_ship(x, cur_index, resource, renderer) { cur_index += 1 }
            }
        }
        let size_count = gameboard.size_count.clone();
        gameboard.update_ship_counter(&size_count, renderer, board_index);
    }

    pub fn find_ship(&mut self, index: i32) -> Option<usize> {
        if let Some(location) = self.ship.iter().position(|ship| ship.index == index) {
            Some(location)
        } else {
            None
        }
    }

    pub fn count_size(&mut self, ship_index: i32) -> i32 {
        let mut count = 0;
        for data in self.data {
            if let BoardType::Ship(index) = data {
                if index == ship_index { count += 1; }
            } else if let BoardType::Hit(index) = data {
                if index == ship_index { count += 1; }
            }
        }
        count
    }

    pub fn hit_place(&mut self, pos: &Vec2, resource: &TextureAllocation, renderer: &mut GpuRenderer, animation: &mut Animation, gameboard: &mut GameBoard) -> Option<bool> {
        let mut result = None;
        if pos.x >= 0.0 && pos.x < BOARD_SIZE && pos.y >= 0.0 && pos.y < BOARD_SIZE {
            let tile_index = get_tile_pos(pos.x as i32, pos.y as i32);
            if let BoardType::Ship(index) = self.data[tile_index] {
                let ship_index = self.find_ship(index).unwrap();
                self.data[tile_index] = BoardType::Hit(index);

                if self.ship[ship_index].damage_ship() {
                    if self.count_ship() <= 0 {
                        result = Some(true);
                    } else {
                        result = Some(false);
                    }
                    let ship_size = self.count_size(index) - 1;
                    let current_turn = if gameboard.current_turn == 0 { 1 } else { 0 };
                    gameboard.reduce_ship_counter(ship_size as usize, renderer, current_turn)
                } else {
                    result = Some(false);
                }

                let sprite_pos = tile_to_render_pos(pos, self.map.pos, 20.0);
                let mut icon = Image::new(Some(resource.icon_texture), renderer, 1);
                icon.pos = Vec3::new(sprite_pos.x, sprite_pos.y, ICON_ORDER);
                icon.hw = Vec2::new(20.0, 20.0);
                icon.uv = Vec4::new(0.0, 0.0, 20.0, 20.0);
                icon.color = Color::rgba(255, 255, 255, 255);
                self.icon.push(icon);
                animation.play(Vec2::new(sprite_pos.x - 20.0, sprite_pos.y - 20.0));
            } else if self.data[tile_index] == BoardType::None {
                self.data[tile_index] = BoardType::Missed;
                let sprite_pos = tile_to_render_pos(pos, self.map.pos, 20.0);
                let mut icon = Image::new(Some(resource.icon_texture), renderer, 1);
                icon.pos = Vec3::new(sprite_pos.x, sprite_pos.y, ICON_ORDER);
                icon.hw = Vec2::new(20.0, 20.0);
                icon.uv = Vec4::new(20.0, 0.0, 20.0, 20.0);
                icon.color = Color::rgba(255, 255, 255, 255);
                self.icon.push(icon);

                result = Some(false);
            }
        }
        result
    }
}

pub fn in_map_pos(pos: &Vec2, boards: &[Board], screen_size: &PhysicalSize<f32>) -> Option<Vec2> {
    if boards.is_empty() { return None; }

    let mouse_pos = world_to_sprite_2pos(pos, screen_size);

    let board_result = boards.iter().find(|board| {
        (mouse_pos.x) >= board.map.pos.x
            && (mouse_pos.x) <= board.map.pos.x + (board.map.tilesize * BOARD_SIZE as u32) as f32
            && (mouse_pos.y) >= board.map.pos.y
            && (mouse_pos.y) <= board.map.pos.y + (board.map.tilesize * BOARD_SIZE as u32) as f32
    });
    
    let board_data = board_result?;
    let tile_pos = mouse_pos - Vec2::new(board_data.map.pos.x, board_data.map.pos.y);
    Some(Vec2::new((tile_pos.x / board_data.map.tilesize as f32).floor(), (tile_pos.y / board_data.map.tilesize as f32).floor()))
}

pub fn find_map_by_pos(pos: &Vec2, boards: &[Board], screen_size: &PhysicalSize<f32>) -> Option<u32> {
    let mouse_pos = world_to_sprite_2pos(pos, screen_size);
    let mut result: u32 = 0;

    for i in 0..=boards.len() {
        if (mouse_pos.x) >= boards[i].map.pos.x
            && (mouse_pos.x) <= boards[i].map.pos.x + (boards[i].map.tilesize * BOARD_SIZE as u32) as f32
            && (mouse_pos.y) >= boards[i].map.pos.y
            && (mouse_pos.y) <= boards[i].map.pos.y + (boards[i].map.tilesize * BOARD_SIZE as u32) as f32 {
                result = i as u32;
                break;
            }
    }
    Some(result)
}