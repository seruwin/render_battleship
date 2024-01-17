#![allow(dead_code, clippy::collapsible_match, unused_imports)]
use backtrace::Backtrace;
use camera::{
    controls::{Controls, FlatControls, FlatSettings},
    Projection,
};
use cosmic_text::{Attrs, Metrics};
use ::futures::SinkExt;
use glam::vec4;
use graphics::{ *};
use hecs::World;
use input::{Bindings, FrameTime, InputHandler};
use log::{error, info, warn, Level, LevelFilter, Metadata, Record};
use naga::{front::wgsl, valid::Validator};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::HashMap,
    fs::{self, File},
    io::{prelude::*, Read, Write},
    iter, panic,
    path::PathBuf,
    rc::Rc,
    time::Duration,
};
use wgpu::{Backends, Dx12Compiler, InstanceDescriptor, InstanceFlags};
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

//use graphics::iced_wgpu::{Backend, Renderer, Settings};
/*use graphics::iced_winit::{
    conversion,
    core::{mouse, renderer, Color as iced_color, Size},
    futures,
    runtime::{program, Debug},
    style::Theme,
    winit, Clipboard,
};*/

use std::collections::HashSet;

use rand::Rng;

mod gamestate;
mod ui;

use gamestate::*;

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
enum Action {
    Quit,
    Select,
}

const ACTION_SIZE: usize = 2;
const SCREEN_ZOOM: f32 = 2.0;

const BOARD_SIZE: f32 = 12.0;

const SHIP_ORDER: f32 = 3.2;
const ICON_ORDER: f32 = 3.1;
const EXPLOSION_ORDER: f32 = 3.0;
const GUI_BG_ORDER: f32 = 2.2;
const GUI_SHADE_ORDER: f32 = 2.1;
const GUI_RESULT_ORDER: f32 = 2.0;

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
enum Axis {
    Forward,
    Sideward,
    Yaw,
    Pitch,
}

// creates a static global logger type for setting the logger
static MY_LOGGER: MyLogger = MyLogger(Level::Debug);

struct MyLogger(pub Level);

struct TextureAllocation {
    ship_texture: Allocation,
    icon_texture: Allocation,
    explosion_texture: Allocation,
    game_bg_texture: Allocation,
    result_texture: Allocation,
    white_texture: Allocation,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum BoardType {
    None,
    Ship(i32),
    Hit(i32),
    Missed,
}

struct Ship {
    sprite: Image,
    parts: i32,
    index: i32,
    visible: bool,
}

impl Ship {
    fn new(resource: &TextureAllocation, renderer: &mut GpuRenderer, index: i32, parts: i32) -> Self {
        Self {
            sprite: Image::new(Some(resource.ship_texture), renderer, 1),
            parts,
            index,
            visible: false,
        }
    }

    fn damage_ship(&mut self) -> bool {
        self.parts -= 1;
        if self.parts <= 0 {
            self.visible = true;
            true
        } else {
            false
        }
    }
}

struct GameBoard {
    got_winner: bool,
    current_turn: i32,
    size_count: [i32; 4],
    win_image: Image,
    lose_image: Image,
    board_shade: [Image; 2],
    status_text: Text,
}

impl GameBoard {
    fn new(resource: &TextureAllocation, renderer: &mut GpuRenderer, scale: &f64) -> Self {
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
        };
        result.status_text.set_buffer_size(renderer, renderer.size().width as i32, renderer.size().height as i32)
            .set_bounds(Some(Bounds::new(348.0, 0.0, 746.0, 20.0)))
            .set_default_color(Color::rgba(185, 185, 185, 255));
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

    fn set_winner(&mut self, index: i32) {
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

    fn change_status_text(&mut self, message: &str, renderer: &mut GpuRenderer) {
        self.status_text.set_text(
            renderer,
            message,
            Attrs::new(),
        );
    }
}

struct Board {
    data: [BoardType; 256],
    ship: Vec<Ship>,
    icon: Vec<Image>,
    map: Map,
}

impl Board {
    fn new(renderer: &mut GpuRenderer, pos: Vec2) -> Self {
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

    fn count_ship(&mut self) -> i32 {
        let mut ship_index: Vec<i32>;
        ship_index = Vec::with_capacity(1);
        for data in self.data {
            if let BoardType::Ship(index) = data { ship_index.push(index); }
        }
        let set_data: HashSet<_> = ship_index.into_iter().collect();
        set_data.len() as i32
    }

    fn check_vertical(&mut self, pos: &Vec2, size: usize) -> bool {
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
    
    fn check_horizontal(&mut self, pos: &Vec2, size: usize) -> bool {
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
    
    fn calculate_available_tile(&mut self, size: usize) -> Vec<i32> {
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
    
    fn try_place_ship(&mut self, index: i32, size: usize) -> Option<(i32, Vec2)> {
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
    
    fn place_ship(&mut self, size: usize, index: i32, resource: &TextureAllocation, renderer: &mut GpuRenderer) -> bool {
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

    fn prepare_board(&mut self, resource: &TextureAllocation, renderer: &mut GpuRenderer, gameboard: &GameBoard) {
        let mut cur_index = 0;
        for x in 0..4 {
            for _i in 0..=gameboard.size_count[x] {
                if self.place_ship(x, cur_index, resource, renderer) { cur_index += 1 }
            }
        }
    }

    fn find_ship(&mut self, index: i32) -> Option<usize> {
        if let Some(location) = self.ship.iter().position(|ship| ship.index == index) {
            Some(location)
        } else {
            None
        }
    }

    fn hit_place(&mut self, pos: &Vec2, resource: &TextureAllocation, renderer: &mut GpuRenderer, animation: &mut Animation) -> Option<bool> {
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

struct Animation {
    sprite: Image,
    in_play: bool,
    frame: i32,
    max_frame: i32,
    elapsed_time: f32,
}

impl Animation {
    fn new(resource: &TextureAllocation, renderer: &mut GpuRenderer) -> Self {
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

    fn update_frame(&mut self, seconds: f32) {
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

    fn play(&mut self, pos: Vec2) {
        if !self.in_play {
            self.sprite.pos = Vec3::new(pos.x, pos.y, EXPLOSION_ORDER);
            self.sprite.changed = true;
            self.in_play = true;
        }
    }
}

impl log::Log for MyLogger {
    // checks if it can log these types of events.
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.0
    }

    // This logs to a panic file. This is so we can see
    // Errors and such if a program crashes in full render mode.
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let msg = format!("{} - {}\n", record.level(), record.args());
            println!("{}", &msg);

            let mut file = match File::options()
                .append(true)
                .create(true)
                .open("paniclog.txt")
            {
                Ok(v) => v,
                Err(_) => return,
            };

            let _ = file.write(msg.as_bytes());
        }
    }
    fn flush(&self) {}
}

fn action_index(action: Action) -> usize {
    match action {
        Action::Quit => 0,
        Action::Select => 1,
    }
}

fn get_tile_pos(x: i32, y: i32) -> usize {
    (x + (y * BOARD_SIZE as i32)) as usize
}

fn find_x_base_on_tile(tile: usize) -> usize {
    tile % BOARD_SIZE as usize
}

fn find_y_base_on_tile(tile: usize) -> usize {
    tile / BOARD_SIZE as usize
}

fn world_to_sprite_3pos(pos: &Vec3, size: &PhysicalSize<f32>) -> Vec3 {
    Vec3::new(pos.x / SCREEN_ZOOM, (size.height - pos.y) / SCREEN_ZOOM, pos.z)
}

fn world_to_sprite_2pos(pos: &Vec2, size: &PhysicalSize<f32>) -> Vec2 {
    Vec2::new(pos.x / SCREEN_ZOOM, (size.height - pos.y) / SCREEN_ZOOM)
}

fn in_map_pos(pos: &Vec2, boards: &[Board], screen_size: &PhysicalSize<f32>) -> Option<Vec2> {
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

fn find_map_by_pos(pos: &Vec2, boards: &[Board], screen_size: &PhysicalSize<f32>) -> Option<u32> {
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

fn tile_to_render_pos(pos: &Vec2, map_start_pos: Vec2, tile_size: f32) -> Vec2 {
    Vec2::new(map_start_pos.x + (pos.x * tile_size), map_start_pos.y + (pos.y * tile_size))
}

#[tokio::main]
async fn main() -> Result<(), AscendingError> {
    // Create logger to output to a File
    log::set_logger(&MY_LOGGER).unwrap();
    // Set the Max level we accept logging to the file for.
    log::set_max_level(LevelFilter::Info);

    info!("starting up");

    // This allows us to take control of panic!() so we can send it to a file via the logger.
    panic::set_hook(Box::new(|panic_info| {
        let bt = Backtrace::new();
        error!("PANIC: {}, BACKTRACE: {:?}", panic_info, bt);
    }));

    // Starts an event gathering type for the window.
    let event_loop = EventLoop::new();

    // Builds the Windows that will be rendered too.
    let window = WindowBuilder::new()
        .with_title("Game")
        .with_inner_size(PhysicalSize::new(1096, 560))
        .with_visible(false)
        .with_resizable(false)
        .with_maximized(false)
        .build(&event_loop)
        .unwrap();

    // Generates an Instance for WGPU. Sets WGPU to be allowed on all possible supported backends
    // These are DX12, DX11, Vulkan, Metal and Gles. if none of these work on a system they cant
    // play the game basically.
    let instance: wgpu::Instance = wgpu::Instance::new(InstanceDescriptor {
        backends: Backends::all(),
        flags: InstanceFlags::default(),
        dx12_shader_compiler: Dx12Compiler::default(),
        gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
    });

    // This is used to ensure the GPU can load the correct.
    let compatible_surface =
        unsafe { instance.create_surface(&window).unwrap() };
    print!("{:?}", &compatible_surface);

    // This creates the Window Struct and Device struct that holds all the rendering information
    // we need to render to the screen. Window holds most of the window information including
    // the surface type. device includes the queue and GPU device for rendering.
    // This then adds gpu_window and gpu_device and creates our renderer type. for easy passing of window, device and font system.
    let mut renderer = instance
        .create_device(
            window,
            &wgpu::RequestAdapterOptions {
                // High performance mode says to use Dedicated Graphics devices first.
                // Low power is APU graphic devices First.
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&compatible_surface),
                // we will never use this as this forces us to use an alternative renderer.
                force_fallback_adapter: false,
            },
            // used to deturmine if we need special limits or features for our backends.
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::default(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None,
            // How we are presenting the screen which causes it to either clip to a FPS limit or be unlimited.
            wgpu::PresentMode::AutoVsync,
        )
        .await
        .unwrap();

    // we print the GPU it decided to use here for testing purposes.
    println!("{:?}", renderer.adapter().get_info());

    // We generate Texture atlases to use with out types.
    let mut atlases: Vec<AtlasGroup> = iter::from_fn(|| {
        Some(AtlasGroup::new(
            &mut renderer,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ))
    })
    .take(4)
    .collect();

    // we generate the Text atlas seperatly since it contains a special texture that only has the red color to it.
    // and another for emojicons.
    let text_atlas = TextAtlas::new(&mut renderer).unwrap();

    // We establish the different renderers here to load their data up to use them.
    let text_renderer = TextRenderer::new(&renderer).unwrap();
    let sprite_renderer = ImageRenderer::new(&renderer).unwrap();
    let map_renderer = MapRenderer::new(&mut renderer, 81).unwrap();

    // get the screen size.
    let mut size = renderer.size();

    // setup our system which includes Camera and projection as well as our controls.
    // for the camera.
    let system = System::new(
        &mut renderer,
        Projection::Orthographic {
            left: 0.0,
            right: size.width,
            bottom: 0.0,
            top: size.height,
            near: 1.0,
            far: -100.0,
        },
        FlatControls::new(FlatSettings { zoom: SCREEN_ZOOM }),
        [size.width, size.height],
    );

    // Create the mouse/keyboard bindings for our stuff.
    let mut bindings = Bindings::<Action, Axis>::new();
    bindings.insert_action(
        Action::Quit,
        vec![winit::event::VirtualKeyCode::Q.into()],
    );

    // set bindings and create our own input handler.
    let mut input_handler = InputHandler::new(bindings);

    // get the Scale factor the pc currently is using for upscaling or downscaling the rendering.
    let scale = renderer.window().current_monitor().unwrap().scale_factor();

    // This is how we load a image into a atlas/Texture. It returns the location of the image
    // within the texture. its x, y, w, h.  Texture loads the file. group_uploads sends it to the Texture
    // renderer is used to upload it to the GPU when done.
    let resource = TextureAllocation {
        ship_texture: Texture::from_file("images/entity/e1.png")?
            .group_upload(&mut atlases[0], &renderer)
            .ok_or_else(|| OtherError::new("failed to upload image"))?,
        icon_texture: Texture::from_file("images/entity/e2.png")?
            .group_upload(&mut atlases[0], &renderer)
            .ok_or_else(|| OtherError::new("failed to upload image"))?,
        explosion_texture: Texture::from_file("images/animation/a2.png")?
            .group_upload(&mut atlases[0], &renderer)
            .ok_or_else(|| OtherError::new("failed to upload image"))?,
        game_bg_texture: Texture::from_file("images/gui/game_bg.png")?
            .group_upload(&mut atlases[0], &renderer)
            .ok_or_else(|| OtherError::new("failed to upload image"))?,
        result_texture: Texture::from_file("images/gui/result.png")?
            .group_upload(&mut atlases[0], &renderer)
            .ok_or_else(|| OtherError::new("failed to upload image"))?,
        white_texture: Texture::from_file("images/white.png")?
            .group_upload(&mut atlases[0], &renderer)
            .ok_or_else(|| OtherError::new("failed to upload image"))?,
    };
    let _tilesheet = Texture::from_file("images/tiles/1.png")?
        .new_tilesheet(&mut atlases[1], &renderer, 20)
        .ok_or_else(|| OtherError::new("failed to upload tiles"))?;

    // Create Board data
    let mut gameboard = GameBoard::new(&resource, &mut renderer, &scale);
    let mut boards = [
        Board::new(&mut renderer, Vec2::new(27.0, 11.0)),
        Board::new(&mut renderer, Vec2::new(297.0, 11.0)),
    ];
    boards.iter_mut().for_each(|board| {
        board.prepare_board(&resource, &mut renderer, &gameboard);
    });

    // Setup Manual Animation
    let mut animation = Animation::new(&resource, &mut renderer);

    // GUI
    let mut guis = Vec::with_capacity(1);
    let mut gui = Image::new(Some(resource.game_bg_texture), &mut renderer, 1);
    gui.pos = Vec3::new(0.0, 0.0, GUI_BG_ORDER);
    gui.hw = Vec2::new(548.0, 280.0);
    gui.uv = Vec4::new(0.0, 0.0, 548.0, 280.0);
    gui.color = Color::rgba(255, 255, 255, 255);
    guis.push(gui);

    // create a Text rendering object.
    let mut text = Text::new(
        &mut renderer,
        Some(Metrics::new(16.0, 16.0).scale(scale as f32)),
        Vec3::new(5.0, 5.0, 0.0),
        Vec2::new(100.0, 16.0),
    );
    text.set_buffer_size(&mut renderer, size.width as i32, size.height as i32)
        .set_bounds(Some(Bounds::new(0.0, 0.0, 100.0, 21.0)))
        .set_default_color(Color::rgba(255, 255, 255, 255));

    // Allow the window to be seen. hiding it then making visible speeds up
    // load times.
    renderer.window().set_visible(true);

    // add everything into our convience type for quicker access and passing.
    let mut state = State {
        system,
        guis,
        image_atlas: atlases.remove(0),
        sprite_renderer,
        text_atlas,
        text_renderer,
        map_renderer,
        map_atlas: atlases.remove(0),
    };

    let mut frame_time = FrameTime::new();
    let mut time = 0.0f32;
    let mut fps = 0u32;

    // Buttons
    let mut did_key_press = [false; ACTION_SIZE];

    #[allow(deprecated)]
    event_loop.run(move |event, _, control_flow| {
        // we check for the first batch of events to ensure we dont need to stop rendering here first.
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
                ..
            } if window_id == renderer.window().id() => {
                if let WindowEvent::CloseRequested = *event {
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::MainEventsCleared => {
                renderer.window().request_redraw();
            }
            _ => {}
        }

        // get the current window size so we can see if we need to resize the renderer.
        let new_size = renderer.size();
        let inner_size = renderer.window().inner_size();

        // if our rendering size is zero stop rendering to avoid errors.
        if new_size.width == 0.0
            || new_size.height == 0.0
            || inner_size.width == 0
            || inner_size.height == 0
        {
            return;
        }

        // update our inputs.
        input_handler.update(renderer.window(), &event, 1.0);

        // update our renderer based on events here
        if !renderer.update(&event).unwrap() {
            return;
        }

        if size != new_size {
            size = new_size;

            // Reset screen size for the Surface here.
            state.system.set_projection(Projection::Orthographic {
                left: 0.0,
                right: new_size.width,
                bottom: 0.0,
                top: new_size.height,
                near: 1.0,
                far: -100.0,
            });

            renderer.update_depth_texture();
        }

        let seconds = frame_time.seconds();

        // check if out close action was hit for esc
        if input_handler.is_action_down(&Action::Quit) {
            *control_flow = ControlFlow::Exit;
        }
        // Check input
        if input_handler.is_mouse_button_down(MouseButton::Left) {
            if !did_key_press[action_index(Action::Select)] {
                did_key_press[action_index(Action::Select)] = true;
                
                let mouse_pos = input_handler.mouse_position().unwrap();
                let tile_pos = in_map_pos(&Vec2::new(mouse_pos.0, mouse_pos.1), &boards, &size);
                
                if !tile_pos.is_none() && !gameboard.got_winner {
                    let board_data = find_map_by_pos(&Vec2::new(mouse_pos.0, mouse_pos.1), &boards, &size);
                    if !board_data.is_none() {
                        let board_index = board_data.unwrap() as usize;
                        if gameboard.current_turn != board_index as i32 {
                            let hit_result = boards[board_index].hit_place(&tile_pos.unwrap(), &resource, &mut renderer, &mut animation);
                            if !hit_result.is_none() {
                                let got_winner = hit_result.unwrap();
                                if got_winner {
                                    gameboard.set_winner(gameboard.current_turn);
                                    gameboard.change_status_text("", &mut renderer);
                                } else {
                                    gameboard.current_turn = board_index as i32;
                                    gameboard.change_status_text(&format!("PLAYER {} TURN", gameboard.current_turn + 1), &mut renderer);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            did_key_press[action_index(Action::Select)] = false;
        }

        // Handle Manual Animation
        animation.update_frame(seconds);

        // update our systems data to the gpu. this is the Camera in the shaders.
        state.system.update(&renderer, &frame_time);
        // update our systems data to the gpu. this is the Screen in the shaders.
        state.system.update_screen(&renderer, [new_size.width, new_size.height]);

        // This adds the Image data to the Buffer for rendering.
        // GUI
        state.guis.iter_mut().for_each(|gui| {
            state.sprite_renderer.image_update(gui, &mut renderer);
        });
        if gameboard.got_winner {
            state.sprite_renderer.image_update(&mut gameboard.board_shade[0], &mut renderer);
            state.sprite_renderer.image_update(&mut gameboard.board_shade[1], &mut renderer);
            state.sprite_renderer.image_update(&mut gameboard.win_image, &mut renderer);
            state.sprite_renderer.image_update(&mut gameboard.lose_image, &mut renderer);
        } else {
            state.sprite_renderer.image_update(&mut gameboard.board_shade[gameboard.current_turn as usize], &mut renderer);
        }
        // Animation
        if animation.in_play { state.sprite_renderer.image_update(&mut animation.sprite, &mut renderer); }
        // Board
        boards.iter_mut().for_each(|board| {
            board.ship.iter_mut().for_each(|ship| {
                if ship.visible {
                    state.sprite_renderer.image_update(&mut ship.sprite, &mut renderer);
                }
            });
            board.icon.iter_mut().for_each(|icon| {
                state.sprite_renderer.image_update(icon, &mut renderer);
            });
            state.map_renderer.map_update(&mut board.map, &mut renderer);
        });
        // Text
        state.text_renderer.text_update(&mut text, &mut state.text_atlas, &mut renderer).unwrap();
        state.text_renderer.text_update(&mut gameboard.status_text, &mut state.text_atlas, &mut renderer).unwrap();
        // this cycles all the Image's in the Image buffer by first putting them in rendering order
        // and then uploading them to the GPU if they have moved or changed in any way. clears the
        // Image buffer for the next render pass. Image buffer only holds the ID's and Sortign info
        // of the finalized Indicies of each Image.
        state.sprite_renderer.finalize(&mut renderer);
        state.map_renderer.finalize(&mut renderer);
        state.text_renderer.finalize(&mut renderer);

        // Start encoding commands. this stores all the rendering calls for execution when
        // finish is called.
        let mut encoder = renderer.device().create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("command encoder"),
            },
        );
        // Run the render pass. for the games renderer
        state.render(&renderer, &mut encoder);

        // Submit our command queue. for it to upload all the changes that were made.
        // Also tells the system to begin running the commands on the GPU.
        renderer.queue().submit(std::iter::once(encoder.finish()));

        if time < seconds {
            text.set_text(
                &mut renderer,
                &format!("FPS: {fps}"),
                Attrs::new(),
            );
            fps = 0u32;
            time = seconds + 1.0;
        }

        fps += 1;

        input_handler.end_frame();
        frame_time.update();
        renderer.present().unwrap();

        // These clear the Last used image tags.
        // Can be used later to auto unload things not used anymore if ram/gpu ram becomes a issue.
        state.image_atlas.trim();
        state.text_atlas.trim();
    })
}
