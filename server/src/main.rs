use std::{
    io::{Read, Write},
    net::TcpStream,
    thread::sleep,
    time::Duration,
};

use brightnes_common::serial::SerialRequest;

use crate::{fs::FileSystem, sound::Sound};

const SERVER_ADDR: &str = "127.0.0.1:19837";
const RETRY_DELAY_MS: u64 = 5000;

const SPECIAL_CTRL_CHAR: u8 = 0x93;

fn main() {
    println!("====< BrightNES Server >====\n");

    let mut sound = Sound::new();

    loop {
        let mut stream = connect_to_server();

        println!("[-] Connected to server at {}", SERVER_ADDR);

        if let Err(e) = req_loop(&mut stream, &mut sound) {
            println!("[!] Connection error: {}", e);
        }
    }
}

fn connect_to_server() -> TcpStream {
    loop {
        match TcpStream::connect(SERVER_ADDR) {
            Ok(stream) => return stream,
            Err(e) => {
                println!("[!] Failed to connect to server at {}: {}", SERVER_ADDR, e);
                println!("[*] Retrying in {} ms...", RETRY_DELAY_MS);
                sleep(Duration::from_millis(RETRY_DELAY_MS));
            }
        }
    }
}

fn req_loop(stream: &mut TcpStream, sound: &mut Sound) -> std::io::Result<()> {
    let mut buf = [0; 1];
    loop {
        stream.read_exact(&mut buf)?;
        if buf[0] == SPECIAL_CTRL_CHAR {
            handle_req(stream, sound)?;
        }
    }
}

fn handle_req(stream: &mut TcpStream, sound: &mut Sound) -> std::io::Result<()> {
    let mut buf = [0; 1];
    stream.read_exact(&mut buf)?;
    match buf[0] {
        x if x == SerialRequest::Active as u8 => {
            stream.write_all(&[1])?;

            println!("[ ]  Sent active response.");
        }
        x if x == SerialRequest::SaveState as u8 => {
            let file_name = FileSystem::save_state(stream)?;
            println!("[ ]  Saved state to file: {}", file_name);
        }
        x if x == SerialRequest::LoadState as u8 => {
            FileSystem::load_state(stream)?;
        }
        x if x == SerialRequest::SaveRAM as u8 => {
            let file_name = FileSystem::save_ram(stream)?;
            println!("[ ]  Saved RAM to file: {}", file_name);
        }
        x if x == SerialRequest::LoadRAM as u8 => {
            FileSystem::load_ram(stream)?;
        }
        x if x == SerialRequest::Sound as u8 => {
            sound.receive_request(stream)?;
        }
        _ => {
            println!("[!] Unknown request: {}", buf[0]);
        }
    }
    Ok(())
}

pub mod fs;
pub mod sound;
