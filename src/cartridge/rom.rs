use derive_builder::Builder;
use log::{info, warn};
use std::fmt::Display;
use thiserror::Error;

use crate::cartridge::MbcType;

pub struct Rom {
    data: Vec<u8>,
    title: String,
    manufacturer_code: [u8; 4],
    cgb_flag: CgbFlag,
    new_licensee_code: [u8; 2],
    sgb_flag: bool,
    cartridge_type: CartridgeType,
    rom_size: usize,
    ram_size: usize,
    destination_code: String,
    old_licensee_code: u8,
    mask_rom_version: u8,
    header_checksum: u8,
    global_checksum: u16,
}

impl Rom {
    pub fn new(data: &[u8]) -> Result<Self, RomError> {
        let title = data[0x0134..=0x0143]
            .iter()
            .copied()
            .take_while(|&c| c != 0)
            .filter(|&c| c.is_ascii())
            .map(|c| c as char)
            .collect::<String>();
        let manufacturer_code = data[0x013F..=0x0142].try_into().unwrap();

        let cgb_flag = match data[0x0143] {
            0x80 => CgbFlag::DualCompatible,
            0xC0 => CgbFlag::CgbOnly,
            _ => CgbFlag::DMGOnly,
        };
        let new_licensee_code = data[0x0144..=0x0145].try_into().unwrap();
        let sgb_flag = data[0x0146] == 0x03;
        let cartridge_type = CartridgeType::new(data[0x0147])?;
        let rom_size = match data[0x0148] {
            0x00 => 32 * 1024,
            0x01 => 64 * 1024,
            0x02 => 128 * 1024,
            0x03 => 256 * 1024,
            0x04 => 512 * 1024,
            0x05 => 1024 * 1024,
            0x06 => 2 * 1024 * 1024,
            0x07 => 4 * 1024 * 1024,
            0x08 => 8 * 1024 * 1024,
            _ => return Err(RomError::InvalidRomSize(data[0x0148])),
        };

        let ram_size = match data[0x0149] {
            0x00 => 0,
            0x01 => 2 * 1024,
            0x02 => 8 * 1024,
            0x03 => 32 * 1024,
            0x04 => 128 * 1024,
            0x05 => 64 * 1024,
            _ => return Err(RomError::InvalidRamSize(data[0x0149])),
        };

        let destination_code = match data[0x014A] {
            0x00 => "Japanese",
            _ => "Overseas Only",
        };
        let old_licensee_code = data[0x014B];
        let mask_rom_version = data[0x014C];

        let mut header_checksum: u8 = 0;
        for &byte in &data[0x0134..=0x014C] {
            header_checksum = header_checksum.wrapping_sub(byte).wrapping_sub(1);
        }
        if header_checksum != data[0x014D] {
            warn!("Invalid header checksum");
        }

        let mut global_checksum: u16 = 0;
        for (i, byte) in data.iter().enumerate() {
            if !(0x014E..=0x014F).contains(&i) {
                global_checksum = global_checksum.wrapping_add(*byte as u16);
            }
        }

        if global_checksum != u16::from_be_bytes(data[0x014E..=0x014F].try_into().unwrap()) {
            warn!("Invalid global checksum");
        }

        info!("Title: {}", title);
        info!("Manufacturer Code: {:?}", manufacturer_code);
        info!("CGB Flag: {:?}", cgb_flag);
        info!("New Licensee Code: {:?}", new_licensee_code);
        info!("SGB Flag: {}", sgb_flag);
        info!("Cartridge Type: {}", cartridge_type);
        info!("ROM Size: {} bytes", rom_size);
        info!("RAM Size: {} bytes", ram_size);
        info!("Destination Code: {}", destination_code);
        info!("Old Licensee Code: {}", old_licensee_code);
        info!("Mask ROM Version: {}", mask_rom_version);
        info!("Header Checksum: {}", header_checksum);
        info!("Global Checksum: {}", global_checksum);

        Ok(Self {
            data: data.to_vec(),
            title,
            manufacturer_code,
            cgb_flag,
            new_licensee_code,
            sgb_flag,
            cartridge_type,
            rom_size,
            ram_size,
            destination_code: destination_code.to_string(),
            old_licensee_code,
            mask_rom_version,
            header_checksum,
            global_checksum,
        })
    }

    pub(super) fn mbc_type(&self) -> MbcType {
        self.cartridge_type.mbc
    }

    pub fn cgb_flag(&self) -> CgbFlag {
        self.cgb_flag
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn rom_size(&self) -> usize {
        self.rom_size
    }

    pub fn ram_size(&self) -> usize {
        self.ram_size
    }

    pub fn have_ram(&self) -> bool {
        self.cartridge_type.has_ram
    }

    pub fn title(&self) -> &str {
        &self.title
    }
}

#[derive(Error, Debug)]
pub enum RomError {
    #[error("Could not build CartridgeType: {0}")]
    BuilderError(#[from] CartridgeTypeBuilderError),
    #[error("Invalid CartridgeType: {0}")]
    InvalidCartridgeType(u8),
    #[error("Invalid ROM size: {0}")]
    InvalidRomSize(u8),
    #[error("Invalid RAM size: {0}")]
    InvalidRamSize(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgbFlag {
    DMGOnly,
    DualCompatible,
    CgbOnly,
}

#[derive(Builder, Debug, Default)]
struct CartridgeType {
    code: u8,
    mbc: MbcType,
    #[builder(default = false)]
    has_ram: bool,
    #[builder(default = false)]
    has_battery: bool,
    #[builder(default = false)]
    has_timer: bool,
    #[builder(default = false)]
    has_rumble: bool,
    #[builder(default = false)]
    has_sensor: bool,
}

impl CartridgeType {
    fn new(code: u8) -> Result<Self, RomError> {
        let mut builder = CartridgeTypeBuilder::default();
        let cartridge_type = {
            builder.code(code);
            match code {
                0x00 => builder.mbc(MbcType::RomOnly),
                0x01 => builder.mbc(MbcType::Mbc1),
                0x02 => builder.mbc(MbcType::Mbc1).has_ram(true),
                0x03 => builder.mbc(MbcType::Mbc1).has_ram(true).has_battery(true),
                0x05 => builder.mbc(MbcType::Mbc2),
                0x06 => builder.mbc(MbcType::Mbc2).has_battery(true),
                0x08 => builder.mbc(MbcType::RomOnly).has_ram(true),
                0x09 => builder
                    .mbc(MbcType::RomOnly)
                    .has_ram(true)
                    .has_battery(true),
                0x0B => builder.mbc(MbcType::Mmm01),
                0x0C => builder.mbc(MbcType::Mmm01).has_ram(true),
                0x0D => builder.mbc(MbcType::Mmm01).has_ram(true).has_battery(true),
                0x0F => builder.mbc(MbcType::Mbc3).has_timer(true).has_battery(true),
                0x10 => builder
                    .mbc(MbcType::Mbc3)
                    .has_timer(true)
                    .has_ram(true)
                    .has_battery(true),
                0x11 => builder.mbc(MbcType::Mbc3),
                0x12 => builder.mbc(MbcType::Mbc3).has_ram(true),
                0x13 => builder.mbc(MbcType::Mbc3).has_ram(true).has_battery(true),
                0x19 => builder.mbc(MbcType::Mbc5),
                0x1A => builder.mbc(MbcType::Mbc5).has_ram(true),
                0x1B => builder.mbc(MbcType::Mbc5).has_ram(true).has_battery(true),
                0x1C => builder.mbc(MbcType::Mbc5).has_rumble(true),
                0x1D => builder.mbc(MbcType::Mbc5).has_rumble(true).has_ram(true),
                0x1E => builder
                    .mbc(MbcType::Mbc5)
                    .has_rumble(true)
                    .has_ram(true)
                    .has_battery(true),
                0x20 => builder.mbc(MbcType::Mbc6),
                0x22 => builder.mbc(MbcType::Mbc7).has_sensor(true),
                0xFE => builder.mbc(MbcType::Huc3),
                0xFF => builder.mbc(MbcType::Huc1).has_ram(true).has_battery(true),
                _ => return Err(RomError::InvalidCartridgeType(code)),
            }
        };
        cartridge_type.build().map_err(RomError::BuilderError)
    }
}

impl Display for CartridgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "code: {:02X}, {}{}{}{}{}{}",
            self.code,
            self.mbc,
            if self.has_ram { "+RAM" } else { "" },
            if self.has_battery { "+Battery" } else { "" },
            if self.has_timer { "+Timer" } else { "" },
            if self.has_rumble { "+Rumble" } else { "" },
            if self.has_sensor { "+Sensor" } else { "" },
        )
    }
}
