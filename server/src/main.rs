use std::{
    io::{Read, Write},
    net::TcpStream,
    thread::sleep,
    time::Duration,
};

const SERVER_ADDR: &str = "127.0.0.1:19837";
const RETRY_DELAY_MS: u64 = 5000;

const SPECIAL_CTRL_CHAR: u8 = 0x93;

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum SerialRequest {
    #[allow(dead_code)]
    Active = 1,
    SaveState = 2,
    LoadState = 3,
}

fn main() {
    println!("====< BrightNES Server >====\n");

    loop {
        let mut stream = connect_to_server();

        println!("[-] Connected to server at {}", SERVER_ADDR);

        if let Err(e) = req_loop(&mut stream) {
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

fn req_loop(stream: &mut TcpStream) -> std::io::Result<()> {
    let mut buf = [0; 1];
    loop {
        stream.read_exact(&mut buf)?;
        if buf[0] == SPECIAL_CTRL_CHAR {
            handle_req(stream)?;
        }
    }
}

fn handle_req(stream: &mut TcpStream) -> std::io::Result<()> {
    let mut buf = [0; 1];
    stream.read_exact(&mut buf)?;
    match buf[0] {
        x if x == SerialRequest::Active as u8 => {
            println!("[-] Received {:?}", SerialRequest::Active);

            stream.write_all(&[1])?;

            println!("[-] Sent active response.");
        }
        x if x == SerialRequest::SaveState as u8 => {
            println!("[-] Received {:?}", SerialRequest::SaveState);

            let mut size_buf = [0; 4];
            stream.read_exact(&mut size_buf)?;
            let size = u32::from_le_bytes(size_buf) as usize;

            let mut cpu_state = vec![0; size];
            stream.read_exact(&mut cpu_state)?;

            println!("[-] Received CPU state. ({} bytes)", size);
        }
        _ => {
            println!("[!] Unknown request: {}", buf[0]);
        }
    }
    Ok(())
}
