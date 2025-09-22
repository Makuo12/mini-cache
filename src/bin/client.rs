use core::str;
use std::{io::{self, Write}, net::{IpAddr, Ipv4Addr, SocketAddr}};

use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};
pub mod utils;


pub const HELP_TEXT: &str = r#"
available commands:

fetch commands
  get <key>
      retrieve the value of a key.
  hget <key>
      retrieve all fields and values stored in a hash (like redis hgetall).
  smembers <key>
      retrieve all members of a set.

change commands
  set <key> <value>
      set the value of a key.
  hset <key> <field> <value>
      set the value of a field in a hash.
  sadd <key> <value>
      add a value to a set.

delete commands
  del <key>
      delete a key and its value.
  hdel <key> <field>
      delete a specific field from a hash.
  sremove <key> <value>
      remove a value from a set.

notes:
  - keys are strings.
  - hash fields are stored as key–value pairs.
"#;


#[tokio::main]
async fn main() {
    // utils::file_control::select_folder();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    loop {
        let mut client = match TcpStream::connect(addr).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Connection failed {}", e);
                return
            }
        };
        print!("client=# ");
        io::stdout().flush().unwrap(); 
        let mut input = String::new();
        let _ = io::stdin().read_line(&mut input);
        if input.trim() == "exit" || input.trim() == "q" {
            // We quit the program
            break;
        } else if input.trim() == "help" {
            // We show how to use it
            println!("\n{}", HELP_TEXT);
        } else {
            let mut args: String = String::from("target/fill/client\t");
            let list: Vec<&str> = input.trim().split(' ').collect();
            for data in list {
                if data.trim().is_empty() {
                    continue;
                }
                args.push_str(data);
                args.push('\t');
            }
            match client.write_all(args.as_bytes()).await {
                Ok(_) => {
                },
                Err(e) => {
                    eprintln!("{}", e);
                }
            }
            let mut buffer = [0; 1024];
            let n = match client.read(&mut buffer).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{}", e);
                    0
                }
            };
            if n == 0 {
                eprintln!("server closed connection");
            } else {
                println!("{}", str::from_utf8(&buffer[..n]).unwrap());
            }
        }
    }
}