use dirs::data_dir;
use log::info;
use std::fs;
use std::io;

pub fn save_data(rom_name: &str, sram_data: &[u8]) -> Result<(), io::Error> {
    // Retrieve application data directory "
    let mut save_dir = data_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Failed to find the application data directory",
        )
    })?;
    save_dir.push("rust-gameboycolor"); // Change the directory name to "rust-gameboycolor"

    // Create the directory if it doesn't exist
    fs::create_dir_all(&save_dir)?;

    // Set the path for the save file
    let save_file = save_dir.join(format!("{}.srm", rom_name));

    println!("Saving data to {:?}", save_file);
    fs::write(&save_file, sram_data)?;

    Ok(())
}

pub fn load_save_data(rom_name: &str) -> Result<Option<Vec<u8>>, io::Error> {
    // Retrieve application data directory
    let mut save_dir = data_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Failed to find the application data directory",
        )
    })?;
    save_dir.push("rust-gameboycolor"); // Change the directory name to "rust-gameboycolor"

    // Set the path for the save file
    let save_file = save_dir.join(format!("{}.srm", rom_name));

    // If the save file exists, load the data
    info!("Loading save data from {:?}", save_file);
    match fs::read(&save_file) {
        Ok(data) => Ok(Some(data)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}
