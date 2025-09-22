use std::{
    fs::{File, read, read_dir},
    io::{Read, Write},
    net::TcpStream,
};

use chrono::{DateTime, Local, NaiveDateTime, Utc};

pub struct FileSystem;

impl FileSystem {
    const SAVE_DIR: &str = "saves/";

    fn state_file_name() -> String {
        Local::now()
            .format("saves/%Y%m%d_%H%M%S%.3f.brst")
            .to_string()
    }

    fn ram_file_name() -> String {
        Local::now()
            .format("saves/%Y%m%d_%H%M%S%.3f.brram")
            .to_string()
    }

    fn get_latest_state_file(extension: &str) -> std::io::Result<Option<String>> {
        let mut latest_date = DateTime::<Utc>::MIN_UTC;
        let mut file = None;

        for entry in read_dir(Self::SAVE_DIR)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension()
            {
                if ext == extension {
                    // This is a BrightNES state file.

                    // Try to parse the date from the file name.
                    let file_stem = path.file_stem().unwrap().to_str().unwrap();
                    if let Ok(date) = NaiveDateTime::parse_from_str(file_stem, "%Y%m%d_%H%M%S%.3f")
                    {
                        let date = date.and_utc();
                        if date > latest_date {
                            // Found the latest file.
                            latest_date = date;
                            file = Some(path.to_str().unwrap().to_string());
                        }
                    }
                }
            }
        }

        Ok(file)
    }

    pub fn save_state(stream: &mut TcpStream) -> std::io::Result<String> {
        let file_name = Self::state_file_name();
        let mut state_file = File::create(&file_name)?;

        let mut size_buf = [0; 4];
        stream.read_exact(&mut size_buf)?;
        let size = u32::from_le_bytes(size_buf) as usize;

        let mut cpu_state = vec![0; size];
        stream.read_exact(&mut cpu_state)?;

        println!("[ ]  Received CPU state. ({} bytes)", size);

        state_file.write_all(&size_buf)?;
        state_file.write_all(&cpu_state)?;

        let mut size_buf = [0; 4];
        stream.read_exact(&mut size_buf)?;
        let size = u32::from_le_bytes(size_buf) as usize;

        let mut ppu_state = vec![0; size];
        stream.read_exact(&mut ppu_state)?;

        println!("[ ]  Received PPU state. ({} bytes)", size);

        state_file.write_all(&size_buf)?;
        state_file.write_all(&ppu_state)?;

        let mut size_buf = [0; 4];
        stream.read_exact(&mut size_buf)?;
        let size = u32::from_le_bytes(size_buf) as usize;

        let mut cartridge_state = vec![0; size];
        stream.read_exact(&mut cartridge_state)?;

        println!("[ ]  Received cartridge state. ({} bytes)", size);

        state_file.write_all(&size_buf)?;
        state_file.write_all(&cartridge_state)?;

        println!("[ ]  Received and wrote whole data.");

        Ok(file_name)
    }

    pub fn load_state(stream: &mut TcpStream) -> std::io::Result<()> {
        if let Some(file_name) = Self::get_latest_state_file("brst")? {
            println!("[ ]  Loading state from file: {}", file_name);

            let state = read(file_name)?;
            stream.write_all(&state)?;

            println!("[ ]  Loaded and sent whole data.");

            Ok(())
        } else {
            println!("[!] No state file found.");

            Ok(())
        }
    }

    pub fn save_ram(stream: &mut TcpStream) -> std::io::Result<String> {
        let file_name = Self::ram_file_name();
        let mut ram_file = File::create(&file_name)?;

        let mut size_buf = [0; 4];
        stream.read_exact(&mut size_buf)?;
        let size = u32::from_le_bytes(size_buf) as usize;

        let mut ram = vec![0; size];
        stream.read_exact(&mut ram)?;

        println!("[ ]  Received RAM state. ({} bytes)", size);

        ram_file.write_all(&size_buf)?;
        ram_file.write_all(&ram)?;

        println!("[ ]  Received and wrote whole data.");

        Ok(file_name)
    }

    pub fn load_ram(stream: &mut TcpStream) -> std::io::Result<()> {
        if let Some(file_name) = Self::get_latest_state_file("brram")? {
            println!("[ ]  Loading RAM from file: {}", file_name);

            let ram = read(file_name)?;
            stream.write_all(&ram)?;

            println!("[ ]  Loaded and sent whole data.");

            Ok(())
        } else {
            println!("[!] No RAM file found.");

            Ok(())
        }
    }
}
