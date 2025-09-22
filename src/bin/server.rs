use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, path::PathBuf, sync::Arc};

use bytes::{Bytes};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::{mpsc::{self, Receiver, Sender}, Mutex}};
use utils::{models::Memory, Cache, CacheResult, Command};

use crate::utils::models::Pipe;


pub mod utils;

const DATA_PATH: Option<&str> = option_env!("DATA_PATH");

#[tokio::main]
async fn main() {
    // DATA_PATH IS DEFINED AT COMPILE TIME
    let build_path = match DATA_PATH {
        Some(value) => value,
        None => {
            eprintln!("⚠️  DATA_PATH environment variable not set, using default: ./data");
            eprintln!("   To set a custom path, use: export DATA_PATH=/your/custom/path");
            return
        }
    };
    let full_path = format!("{}/_data.bin", build_path);
    let path = PathBuf::from(full_path);
    
    let memory = match Memory::new(path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}", e);
            return
        }
    };
    
    let resource = Arc::new(Mutex::new(memory));
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let listener = match TcpListener::bind(addr).await {
        Ok(l) => {
            println!("Listing at {}", addr);
            l
        },
        Err(e) => {
            eprintln!("Socket failded {}", e);
            return
        }
    };
    let (tx, rx) = mpsc::channel(100);
    let m_job = resource.clone();
    tokio::spawn(async move {
        update_data_to_file(m_job, rx).await;
    });
    loop {
        let (socket, _) = match listener.accept().await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Stream failed {}", e);
                return
            }
        };
        let tx_new = tx.clone();
        let m = resource.clone();
        tokio::spawn(async move {
            process_stream(socket, m, tx_new).await;
        });
    }
}

async fn process_stream(mut socket: TcpStream, memory: Arc<Mutex<Memory>>, tx: Sender<Pipe>) {
    let mut buffer: Vec<u8> = vec![0;1024];
    let size  = match socket.read(&mut buffer).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Reading failed {}", e);
            return;
        }
    };
    let cmd = match Command::new(size, buffer) {
        Ok(c) => c,
        Err(e) => {
            let _ = socket.write_all(e.show_err()).await;
            let _ = socket.flush().await;
            return;
        }
    };
    let cache = match Cache::new(&cmd) {
        Ok(c) => c,
        Err(e) => {
            let _ = socket.write_all(e.show_err()).await;
            let _ = socket.flush().await;
            return;
        }
    };
    let result = match cache.handle_cmd(cmd, memory, tx).await {
        CacheResult::Success(s) => s,
        CacheResult::Failure(f) => f
    };
    let _ = socket.write_all(result.as_bytes()).await;
    let _ = socket.flush().await;
}

async fn update_data_to_file(memory: Arc<Mutex<Memory>>, mut rx: Receiver<Pipe>) {
    loop {
        if let Some(data) = rx.recv().await {
            let mut memory = memory.lock().await;
            match data {
                Pipe::Delete(value) => {
                    println!("delete: {:?}", value);
                    let result = memory.modify_file(value).await;
                    if result.is_err() {
                        panic!("error")
                    }
                }, 
                Pipe::Recent(value) => {
                    let mut name: Option<Bytes> = Option::None;
                    if let Some(found) = memory.recent.get(&value) {
                        name = Some(found.clone());
                    }
                    if let Some(name) = name.take() {
                        memory.recent_to_file_schedular(
                            value,
                            &name
                        ).await;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // use std::{env};


    use super::*;

    async fn handler(data: String) -> CacheResult {
        let path = PathBuf::from("src/files/_data.bin");
        let memory = Memory::new(path).unwrap();
        let memory = Arc::new(Mutex::new(memory));
        let cmd = match Command::new(data.len(), data.into_bytes()) {
            Ok(c) => c,
            Err(e) => {
                println!("{}", e.show_err_str());
                return CacheResult::Failure(e.to_string());
            }
        };
        let cache = match Cache::new(&cmd) {
            Ok(c) => c,
            Err(e) => {
                println!("{}", e.show_err_str());
                return CacheResult::Failure(e.to_string());
            }
        };
        let (tx, _) = mpsc::channel(100);
        return cache.handle_cmd(cmd, memory, tx).await;
    }
    #[tokio::test]
    async fn process_stream() {
        let data: String = String::from("target/debug/client\tsadd\tjames\tname\tmakuo\tage\t25\t");
        let result = handler(data).await;
        assert!(matches!(result, CacheResult::Success(_)));
        assert!(!matches!(result, CacheResult::Failure(_)));
        let data: String = String::from("target/debug/client\tsmembers\tjames\t");
        let result = handler(data).await;
        assert!(matches!(result, CacheResult::Success(_)));
        assert!(!matches!(result, CacheResult::Failure(_)));
    }
    #[tokio::test]
    async fn process_cmd() {
        let data: String = String::from("target/debug/client\tset\tdo\tmakuo\t");
        let cmd = Command::new(data.len(), data.into_bytes());
        assert!(cmd.is_ok());
    }
    #[tokio::test]
    async fn process_wrong_stream() {
        let data: String = String::from("target/debug/client\tmet\tname\tmakuo\t");
        let cmd = Command::new(data.len(), data.into_bytes()).unwrap();
        let cache = Cache::new(&cmd);
        assert!(cache.is_err());
    }
    #[tokio::test]
    async fn process_good_stream() {
        let data: String = String::from("target/debug/client\tset\tname\tmakuo\t");
        let cmd = Command::new(data.len(), data.into_bytes()).unwrap();
        let cache = Cache::new(&cmd);
        assert!(cache.is_ok());
    }
    #[tokio::test]
    async fn process_good_stream_hset() {
        let data: String = String::from("target/debug/client\thset\tperson2\tname\tmakuo\tage\t25\t");
        let result = handler(data).await;
        let data: String = String::from("target/debug/client\thget\tperson2\t");
        let result_two = handler(data).await;
        assert!(matches!(result, CacheResult::Success(_)));
        assert!(matches!(result_two, CacheResult::Success(_)));
        assert!(!matches!(result, CacheResult::Failure(_)));
        assert!(!matches!(result_two, CacheResult::Failure(_)));
        
    }
}
