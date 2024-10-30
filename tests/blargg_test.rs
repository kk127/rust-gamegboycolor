use rust_gameboycolor::{DeviceMode, GameBoyColor, LinkCable};

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{bail, Result};

struct Cable {
    buffer: Rc<RefCell<Vec<u8>>>,
    completed: Rc<RefCell<Option<Result<()>>>>,
}

impl LinkCable for Cable {
    fn send(&mut self, data: u8) {
        self.buffer.borrow_mut().push(data);
        if self.completed.borrow().is_none() {
            *self.completed.borrow_mut() = blagg_check(&self.buffer.borrow());
        }
    }

    fn try_recv(&mut self) -> Option<u8> {
        None
    }
}

fn blagg_check(buffer: &[u8]) -> Option<Result<()>> {
    const PASS: &[u8] = b"Passed";
    const FAIL: &[u8] = b"Failed";

    if buffer.ends_with(PASS) {
        return Some(Ok(()));
    } else if buffer.ends_with(FAIL) {
        let message = format!("Failed: {}", String::from_utf8_lossy(buffer));
        return Some(Err(anyhow::anyhow!(message)));
    }
    None
}

fn blagg_test(rom_name: &str) -> Result<()> {
    let rom_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cartridge")
        .join(rom_name);
    let rom = std::fs::read(rom_path)?;

    let buffer = Rc::new(RefCell::new(Vec::new()));
    let completed = Rc::new(RefCell::new(None));
    let cable = Cable {
        buffer: buffer.clone(),
        completed: completed.clone(),
    };
    let mut gameboy = GameBoyColor::new(&rom, DeviceMode::GameBoy, Some(Box::new(cable))).unwrap();
    let mut frame = 0;
    while completed.borrow().is_none() && frame < 60 * 60 {
        gameboy.execute_frame();
        frame += 1;
    }

    let completed_ref = completed.borrow();
    match completed_ref.as_ref() {
        Some(Ok(())) => Ok(()),
        Some(Err(e)) => bail!("Test failed: {}", e),
        None => bail!("Test did not complete"),
    }
}

macro_rules! generate_rom_tests {
    ($($test_name:ident, $rom_path:expr),* $(,)?) => {
        $(
            #[test]
            fn $test_name() -> Result<()> {
                blagg_test($rom_path)
            }
        )*
    };
}

generate_rom_tests!(
    test_01_special,
    "01-special.gb",
    test_02_interrupts,
    "02-interrupts.gb",
    test_03_op_sp_hl,
    "03-op sp,hl.gb",
    test_04_op_r_imm,
    "04-op r,imm.gb",
    test_05_op_rp,
    "05-op rp.gb",
    test_06_ld_r_r,
    "06-ld r,r.gb",
    test_07_jr_jp_call_ret_rst,
    "07-jr,jp,call,ret,rst.gb",
    test_08_misc_instrs,
    "08-misc instrs.gb",
    test_09_op_r_r,
    "09-op r,r.gb",
    test_10_bit_ops,
    "10-bit ops.gb",
    test_11_op_a_hl,
    "11-op a,(hl).gb",
);
