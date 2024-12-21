use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, info};
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

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[clap(short, long)]
    listen_port: String,
    #[clap(short, long)]
    send_port: String,
    #[clap(short, long)]
    file_path: String,
    #[clap(short, long)]
    gb: bool,
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();
    let file_path = args.file_path;
    let listen_port = args.listen_port;
    let send_port = args.send_port;

    let device_mode = if args.gb {
        DeviceMode::GameBoy
    } else {
        DeviceMode::GameBoyColor
    };

    let file = std::fs::read(&file_path).unwrap();

    // let cable = Cable { buffer: Vec::new() };
    let network_cable = NetworkCable::new(listen_port, send_port);

    info!("DeviceMode: {:?}", device_mode);
    let mut gameboy_color =
        gameboycolor::GameBoyColor::new(&file, device_mode, Some(Box::new(network_cable)))?;

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
        samples: Some(800),
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

    let mut reverb = Reverb::new(48_000, 400, 0.2);
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

                    _ => {}
                },
                _ => {}
            }
        }

        // let start_time = time::Instant::now();
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        gameboy_color.set_key(key_state);
        gameboy_color.execute_frame();
        for x in 0..160 {
            for y in 0..144 {
                let index = y * 160 + x;
                let color = gameboy_color.frame_buffer()[index];
                let color = Color::RGB(color.0, color.1, color.2);
                canvas.set_draw_color(color);
                canvas
                    .draw_point((x as i32, y as i32))
                    .map_err(|e| anyhow::anyhow!(e))
                    .context("Failed to draw point")?;
            }
        }
        canvas.present();

        let audio_buffer = gameboy_color.audio_buffer();
        while audio_queue.size() > 1600 {
            std::thread::sleep(time::Duration::from_micros(1));
        }

        let audio_buffer = reverb.process_frame(&audio_buffer);

        audio_queue
            .queue_audio(&audio_buffer.iter().flatten().copied().collect::<Vec<i16>>())
            .map_err(|e| anyhow::anyhow!(e))
            .context("Failed to queue audio")?;

        // 60 FPS
        // let elapsed_time = start_time.elapsed();
        // if elapsed_time < time::Duration::from_micros(16666) {
        //     std::thread::sleep(time::Duration::from_micros(16666) - elapsed_time);
        // }
    }

    if let Some(save_data) = gameboy_color.save_data() {
        utils::save_data(gameboy_color.rom_name(), &save_data)?;
    }

    Ok(())
}

struct Reverb {
    delay_buffer_left: Vec<f32>,  // 左チャンネルの遅延バッファ
    delay_buffer_right: Vec<f32>, // 右チャンネルの遅延バッファ
    write_index: usize,           // 書き込み位置
    delay_samples: usize,         // 遅延サンプル数
    decay: f32,                   // 減衰率
}

impl Reverb {
    /// リバーブの初期化
    /// - `sample_rate`: サンプルレート（例: 44100）
    /// - `delay_ms`: リバーブの遅延時間（ミリ秒）
    /// - `decay`: 減衰率（0.0～1.0）
    pub fn new(sample_rate: usize, delay_ms: usize, decay: f32) -> Self {
        let delay_samples = sample_rate * delay_ms / 1000; // 遅延時間をサンプル数に変換
        Reverb {
            delay_buffer_left: vec![0.0; delay_samples],
            delay_buffer_right: vec![0.0; delay_samples],
            write_index: 0,
            delay_samples,
            decay,
        }
    }

    /// フレーム単位でリバーブを適用する（ステレオ対応）
    /// - `input`: フレーム単位の入力シグナル（固定長のスライス, ステレオ形式）
    /// - 戻り値: リバーブが適用された出力シグナル（固定長のベクタ, ステレオ形式）
    pub fn process_frame(&mut self, input: &[[i16; 2]]) -> Vec<[i16; 2]> {
        let mut output = vec![[0; 2]; input.len()]; // 出力バッファを入力と同じ長さで初期化
        for (i, &sample) in input.iter().enumerate() {
            // 左チャンネル処理
            let sample_left_f32 = sample[0] as f32 / i16::MAX as f32; // 正規化
            let read_index_left = (self.write_index + self.delay_buffer_left.len()
                - self.delay_samples)
                % self.delay_buffer_left.len();
            let delayed_sample_left = self.delay_buffer_left[read_index_left];
            let processed_sample_left = sample_left_f32 + delayed_sample_left * self.decay;
            self.delay_buffer_left[self.write_index] = sample_left_f32; // バッファ更新

            // 右チャンネル処理
            let sample_right_f32 = sample[1] as f32 / i16::MAX as f32; // 正規化
            let read_index_right = (self.write_index + self.delay_buffer_right.len()
                - self.delay_samples)
                % self.delay_buffer_right.len();
            let delayed_sample_right = self.delay_buffer_right[read_index_right];
            let processed_sample_right = sample_right_f32 + delayed_sample_right * self.decay;
            self.delay_buffer_right[self.write_index] = sample_right_f32; // バッファ更新

            // 出力にクリッピングして整数化（f32 -> i16）
            output[i][0] = (processed_sample_left * i16::MAX as f32)
                .clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            output[i][1] = (processed_sample_right * i16::MAX as f32)
                .clamp(i16::MIN as f32, i16::MAX as f32) as i16;

            // 書き込み位置を更新
            self.write_index = (self.write_index + 1) % self.delay_buffer_left.len();
        }
        output
    }
}
