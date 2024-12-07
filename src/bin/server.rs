use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, path::PathBuf, sync::Arc, time::Duration};

use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::Mutex, time::interval};
use utils::{models::Memory, Cache, CacheResult, Command};


pub mod utils;

#[tokio::main]
async fn main() {
    let path = PathBuf::from("src/files/_data.bin");
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
    let m_job = resource.clone();
    tokio::spawn(async move {
        update_data_to_file(m_job).await;
    });
    loop {
        let m = resource.clone();
        let (socket, _) = match listener.accept().await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Stream failed {}", e);
                return
            }
        };
        tokio::spawn(async move {
            process_stream(socket, m).await;
        });
    }
}

async fn process_stream(mut socket: TcpStream, memory: Arc<Mutex<Memory>>) {
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
    let result = match cache.handle_cmd(cmd, memory).await {
        CacheResult::Success(s) => s,
        CacheResult::Failure(f) => f
    };
    let _ = socket.write_all(result.as_bytes()).await;
    let _ = socket.flush().await;
}

async fn update_data_to_file(memory: Arc<Mutex<Memory>>) {
    let mut job = interval(Duration::from_secs(10)); 
    loop {
        job.tick().await;
        let mut memory = memory.lock().await;
        if !memory.recent.is_empty() {
            println!("Starting recent");
            println!("Recent before: {:?}", memory.recent);
            memory.recent_to_file_schedular().await;
            println!("items after: {:?}", memory.item);
            println!("Recent after: {:?}", memory.recent);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{env, io::{BufReader, BufWriter}};

    use utils::models::MainError;

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
        return cache.handle_cmd(cmd, memory).await;
    }
    #[tokio::test]
    async fn process_stream() {
        let data: String = String::from("target/debug/client\tsadd\tjames\tname\tmakuo\tage\t25\t");
        let result = handler(data).await;
        // assert!(matches!(result, CacheResult::Success(_)));
        // assert!(!matches!(result, CacheResult::Failure(_)));
        let data: String = String::from("target/debug/client\tsmembers\tjames\t");
        let result = handler(data).await;
        assert!(matches!(result, CacheResult::Success(_)));
        assert!(!matches!(result, CacheResult::Failure(_)));
    }
    #[tokio::test]
    async fn process_cmd() {
        let data: String = String::from("target/debug/client\tset\tname\tmakuo\t");
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
}
