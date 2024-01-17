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

use rand::Rng;

mod gamestate;
mod ui;
mod func;
mod board;
mod collection;

use gamestate::*;
use board::*;
use board::Animation;
use func::*;
use collection::*;

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
enum Action {
    Quit,
    Select,
}

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
