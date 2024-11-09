use anyhow::{Context, Result};
use rust_gameboycolor::utils;
use rust_gameboycolor::{
    gameboycolor, DeviceMode, JoypadKey, JoypadKeyState, LinkCable, NetworkCable,
};
use sdl2::audio;
use sdl2::event::{self, Event};
use sdl2::keyboard::Keycode;
use sdl2::libc::kevent;
use sdl2::pixels::Color;
use std::env;
use std::path::{Path, PathBuf};
use std::time;

struct Cable {
    buffer: Vec<u8>,
}

impl LinkCable for Cable {
    fn send(&mut self, data: u8) {
        self.buffer.push(data);
        // println!("buffer: {:?}", self.buffer);
        // println!("LinkCable send: {:#04X}", data);
    }

    fn try_recv(&mut self) -> Option<u8> {
        None
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let file_path = env::args().nth(1).expect("No file path provided");
    let file = std::fs::read(&file_path).unwrap();

    // 第2引数: listen port
    // 第3引数: send port
    let listen_port = env::args().nth(2).expect("No listen port provided");
    let send_port = env::args().nth(3).expect("No send port provided");

    // let cable = Cable { buffer: Vec::new() };
    let network_cable = NetworkCable::new(listen_port, send_port);

    // let mut gameboy_color =
    //     gameboycolor::GameBoyColor::new(&file, DeviceMode::GameBoy, Some(Box::new(cable)))?;
    let mut gameboy_color =
        gameboycolor::GameBoyColor::new(&file, DeviceMode::GameBoy, Some(Box::new(network_cable)))?;

    let sdl2_context = sdl2::init()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to initialize SDL2")?;

    let video_subsystem = sdl2_context
        .video()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to initialize video subsystem")?;

    let window = video_subsystem
        .window("rust-cgb", 160 * 3, 144 * 3)
        .position_centered()
        .resizable()
        .build()
        .context("Failed to create window")?;

    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .context("Failed to create canvas")?;

    canvas
        .set_logical_size(160, 144)
        .context("Failed to set logical size")?;

    let audio_subsystem = sdl2_context
        .audio()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to initialize SDL2 audio subsystem")?;
    let desired_spec = sdl2::audio::AudioSpecDesired {
        freq: Some(48_000),
        channels: Some(2),
        samples: Some(1024),
    };
    let audio_queue = audio_subsystem
        .open_queue::<i16, _>(None, &desired_spec)
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to open audio queue")?;
    audio_queue
        .queue_audio(&vec![0i16; 1024])
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to queue audio")?;
    audio_queue.resume();

    let mut event_pump = sdl2_context
        .event_pump()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to get event pump")?;

    let mut key_state = JoypadKeyState::new();
    'running: loop {
        // イベント処理
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::Right => key_state.set_key(JoypadKey::Right, true),
                    Keycode::Left => key_state.set_key(JoypadKey::Left, true),
                    Keycode::Up => key_state.set_key(JoypadKey::Up, true),
                    Keycode::Down => key_state.set_key(JoypadKey::Down, true),
                    Keycode::X => key_state.set_key(JoypadKey::A, true),
                    Keycode::Z => key_state.set_key(JoypadKey::B, true),
                    Keycode::Space => key_state.set_key(JoypadKey::Select, true),
                    Keycode::Return => key_state.set_key(JoypadKey::Start, true),
                    _ => {}
                },
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::Right => key_state.set_key(JoypadKey::Right, false),
                    Keycode::Left => key_state.set_key(JoypadKey::Left, false),
                    Keycode::Up => key_state.set_key(JoypadKey::Up, false),
                    Keycode::Down => key_state.set_key(JoypadKey::Down, false),
                    Keycode::X => key_state.set_key(JoypadKey::A, false),
                    Keycode::Z => key_state.set_key(JoypadKey::B, false),
                    Keycode::Space => key_state.set_key(JoypadKey::Select, false),
                    Keycode::Return => key_state.set_key(JoypadKey::Start, false),

                    // Keycode::Right => key |= 1 << 0,
                    // Keycode::Left => key |= 1 << 1,
                    // Keycode::Up => key |= 1 << 2,
                    // Keycode::Down => key |= 1 << 3,
                    // Keycode::X => key |= 1 << 4,
                    // Keycode::Z => key |= 1 << 5,
                    // Keycode::Space => key |= 1 << 6,
                    // Keycode::Return => key |= 1 << 7,

                    // Keycode::Right => key &= !(1 << 0),
                    // Keycode::Left => key &= !(1 << 1),
                    // Keycode::Up => key &= !(1 << 2),
                    // Keycode::Down => key &= !(1 << 3),
                    // Keycode::X => key &= !(1 << 0),
                    // Keycode::Z => key &= !(1 << 1),
                    // Keycode::Space => key &= !(1 << 2),
                    // Keycode::Return => key &= !(1 << 3),
                    _ => {}
                },
                _ => {}
            }
        }

        let start_time = time::Instant::now();
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        gameboy_color.execute_frame();
        gameboy_color.set_key(key_state);
        for x in 0..160 {
            for y in 0..144 {
                let index = y * 160 + x;
                let color = gameboy_color.frame_buffer()[index];
                // Convert the monochrome color to an RGB color
                let color = match color {
                    0xFF => Color::RGB(255, 255, 255),
                    0xAA => Color::RGB(170, 170, 170),
                    0x55 => Color::RGB(85, 85, 85),
                    0x00 => Color::RGB(0, 0, 0),
                    _ => unreachable!(),
                };
                canvas.set_draw_color(color);
                canvas
                    .draw_point((x as i32, y as i32))
                    .map_err(|e| anyhow::anyhow!(e))
                    .context("Failed to draw point")?;
            }
        }
        canvas.present();

        let audio_buffer = gameboy_color.audio_buffer();
        println!("audio_buffer len: {}", audio_buffer.len());
        // while audio_queue.size() > 1024 * 2 {
        //     std::thread::sleep(time::Duration::from_millis(1));
        // }
        audio_queue
            .queue_audio(&audio_buffer.iter().flatten().copied().collect::<Vec<i16>>())
            .map_err(|e| anyhow::anyhow!(e))
            .context("Failed to queue audio")?;

        // 60 FPS
        let elapsed_time = start_time.elapsed();
        if elapsed_time < time::Duration::from_micros(16742) {
            std::thread::sleep(time::Duration::from_micros(16742) - elapsed_time);
        }
    }

    if let Some(save_data) = gameboy_color.save_data() {
        utils::save_data(&gameboy_color.rom_name(), &save_data);
    }

    Ok(())
}
