use anyhow::{Context, Result};
use rust_gameboycolor::{gameboycolor, DeviceMode};
use sdl2::event::{self, Event};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use std::env;
use std::time;

fn main() -> Result<()> {
    env_logger::init();

    let file_path = env::args().nth(1).expect("No file path provided");
    let file = std::fs::read(file_path).unwrap();
    let mut gameboy_color = gameboycolor::GameBoyColor::new(&file, DeviceMode::GameBoy)?;

    let sdl2_context = sdl2::init()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to initialize SDL2")?;

    let video_subsystem = sdl2_context
        .video()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to initialize video subsystem")?;

    let window = video_subsystem
        .window("rust-cgb", 160 * 5, 144 * 5)
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

    let mut event_pump = sdl2_context
        .event_pump()
        .map_err(|e| anyhow::anyhow!(e))
        .context("Failed to get event pump")?;

    'running: loop {
        // イベント処理
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        let start_time = time::Instant::now();
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        gameboy_color.execute_frame();
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

        // 60 FPS
        let elapsed_time = start_time.elapsed();
        if elapsed_time < time::Duration::from_millis(16) {
            std::thread::sleep(time::Duration::from_millis(16) - elapsed_time);
        }
    }

    Ok(())
}
