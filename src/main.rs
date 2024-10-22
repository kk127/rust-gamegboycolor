use std::env;
use rust_gameboycolor::gameboycolor;

fn main() {
    env_logger::init();

    let file_path = env::args().nth(1).expect("No file path provided");
    let file = std::fs::read(file_path).unwrap();
    let rom = gameboycolor::GameBoyColor::new(&file);
}