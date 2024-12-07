use core::str;
use std::{env, net::{IpAddr, Ipv4Addr, SocketAddr}};

use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};
pub mod utils;


#[tokio::main]
async fn main() {
    utils::file_control::select_folder();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let mut client = match TcpStream::connect(addr).await {
        Ok(s) => s,
        Err(e) => {
            println!("Connection failed {}", e);
            return
        }
    };
    let mut args: String = String::new();
    for arg in env::args() {
        if arg.trim().is_empty()  {
            continue;
        }
        let value = format!("{}\t", arg);
        args+=&value;
    }
    match client.write_all(args.as_bytes()).await {
        Ok(_) => {
            println!("message sent");
        },
        Err(e) => {
            println!("{}", e);
        }
    }
    let mut buffer: Vec<u8> = Vec::new();
    let size = client.read_to_end(&mut buffer).await.unwrap();
    println!("size {}", size);
    println!("details: {}", str::from_utf8(&buffer).unwrap())
}