use std::{collections::HashMap, io::{BufRead, BufReader, Write}, path::PathBuf};
use std::fs::{File, OpenOptions};


use bytes::{BufMut, Bytes, BytesMut};
use tokio::{fs::OpenOptions as OpenOptionsTokio, io::AsyncWriteExt};


use std::fmt::{self, Display, Debug};

use super::CacheResult;

// command\tkey\'value\"value\"value\n

pub enum MainError {
    FileReadError(String),
    BadCommandFormat(String),
    FindCacheTypeError(String)
}

impl Display for MainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileReadError(data) => write!(f, "{}", data),
            Self::BadCommandFormat(data) => write!(f, "{}", data),
            Self::FindCacheTypeError(data) => write!(f, "{}", data)
        }
    }
}

impl MainError {
    pub fn show_err(&self) -> &[u8] {
        match self {
            Self::FileReadError(data) => data.as_bytes(),
            Self::BadCommandFormat(data) => data.as_bytes(),
            Self::FindCacheTypeError(data) => data.as_bytes()
        }
    }
    pub fn show_err_str(&self) -> &String {
        match self {
            Self::FileReadError(data) => data,
            Self::BadCommandFormat(data) => data,
            Self::FindCacheTypeError(data) => data
        }
    }
}

impl Debug for MainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileReadError(arg0) => f.debug_tuple("FileReadError").field(arg0).finish(),
            Self::BadCommandFormat(arg0) => f.debug_tuple("BadCommandFormat").field(arg0).finish(),
            Self::FindCacheTypeError(arg0) => f.debug_tuple("FindCacheTypeError").field(arg0).finish()
        }
    }
}

impl std::error::Error for MainError {
}
#[derive(Debug)]
pub struct Position {
    start: usize,
    end: usize
}

#[derive(Debug)]
pub struct Memory {
    pub path: PathBuf,
    pub buffer: BytesMut,
    pub item: HashMap<String, Position>,
    pub recent: HashMap<String, Bytes>
}

impl Memory {
    pub fn new(path: PathBuf) -> Result<Memory, MainError> {
        let mut buffer = String::new();
        let mut item = HashMap::new();
        if path.exists() {
            let file = match File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    // panic!("Error at exists {}", e);
                    return Err(MainError::FileReadError(e.to_string()))
                }
            };
            let mut position: usize = 0;
            let reader = BufReader::new(&file);
            for line in reader.lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(e) => {
                        // panic!("Cannot read line {}", e);
                        return Err(MainError::FileReadError(e.to_string()))
                    }
                };
                if line.trim().is_empty() {
                    continue;
                }
                let length = line.len();
                let mut line_data = line.split("\'");
                let command_key = match line_data.next() {
                    Some(l) => l.to_owned(),
                    None => {
                        // panic!("Could not split line");
                        return Err(MainError::FileReadError(String::from("Could not split line")))
                    }
                };
                let start = position;
                position += length;
                buffer+=&line;
                item.insert(command_key, Position{start, end: position});
            }
        } else {
            let _ = match File::create(&path) {
                Ok(f) => f,
                Err(e) => {
                    // panic!("Error at not exists {}", e);
                    return Err(MainError::FileReadError(e.to_string()))
                }
            };
        }
        let recent: HashMap<String, Bytes> = HashMap::new();
        let mut buf = BytesMut::with_capacity(buffer.len());
        buf.put(buffer.as_bytes());
        Ok(Memory {path, buffer: buf, item, recent })
    }


    pub async fn recent_to_file_schedular(&mut self) {
        let mut file = match OpenOptionsTokio::new().append(true).write(true).open(&self.path).await {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error at reading in file {}", e);
                return;
            }
        };
        let mut data = String::new();
        let collected: Vec<(_,_)> = self.recent.drain().collect();
        for (key, value) in collected {
            if value.is_empty() {
                continue;
            }
            for b in value.slice(..) {
                data.push(b as char);
            }
            self.item.insert(key, Position { start: self.buffer.len(), 
                end: self.buffer.len() + value.len()});
            self.buffer.put(&value[..]);
            data.push('\n');
        }
        let _ = match file.write(&data.as_bytes()).await {
            Ok(s) => s,
            Err(e) => {
                println!("Error at: {}", e);
                return;
            }
        };
        
    }
    pub fn recent_to_file(&mut self) {
        let mut file = OpenOptions::new().append(true).write(true).open(&self.path).unwrap();
        let mut data = String::new();
        let collected: Vec<(_,_)> = self.recent.drain().collect();
        for (_, value) in collected {
            if value.is_empty() {
                continue;
            }
            for b in value.slice(..) {
                data.push(b as char);
            }
            data.push('\n');
        }
        let _ = match file.write(&data.as_bytes()) {
            Ok(s) => s,
            Err(e) => {
                println!("Error at: {}", e);
                return;
            }
        };
        
    }
    pub fn get(&self, mut key_value: String) -> CacheResult {
        if key_value.trim().is_empty() {
            return CacheResult::Failure(key_value+"Key cannot be empty");
        } else if let Some(value) = self.recent.get(&key_value) {
            return CacheResult::Success(self.get_value(key_value, value));
        }
        else if let Some(value) = self.item.get(&key_value) {
            let items = &self.buffer[value.start..value.end];
            return CacheResult::Success(self.get_value(key_value, items));
        }
        key_value.clear();
        return CacheResult::Failure(key_value+"Data not found");
    }
    fn get_value(&self, mut key_value: String, value: &[u8]) -> String {
        key_value.clear();
        let mut memory_type = MemoryType::Command;
        for item in value {
            if *item == b'\'' {
                memory_type = MemoryType::Value
            } else if memory_type == MemoryType::Value {
                if *item == b'\"' {
                    key_value.push(' ');
                } else {
                    key_value.push(*item as char);
                }
            }
        }
        return key_value;
    }
    pub fn set(&mut self, key: String, value: String, mut action: String) -> CacheResult {
        // First check if the key exist
        for command_key in self.item.keys() {
            let mut split = command_key.split("\t");
            split.next();
            if split.next() == Some(&key) {
                return CacheResult::Failure(String::from("Key already exist. Try another kind"))
            } 
        }
        for command_key in self.recent.keys() {
            let mut split = command_key.split("\t");
            split.next();
            if split.next() == Some(&key) {
                return CacheResult::Failure(String::from("Key already exist. Try another kind"))
            } 
        }
        action = action+"\t";
        action = action+&key;
        self.recent.insert(action, Bytes::from(value));
        return CacheResult::Success(String::from("1"));
    }
}

#[derive(PartialEq)]
enum MemoryType {
    Command,
    Key,
    Value
}

impl Drop for Memory {
    fn drop(&mut self) {
        self.recent_to_file();
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[tokio::test]
    async fn create_memory() {
        // How the data is stored on disk
        // command\tkey\'value\"value\"value\n
        let path =  PathBuf::from("src/files/_data.bin");
        let mut memory = Memory::new(path).unwrap();
        // memory.load_recent(value);
        // memory.write_file(value).await;
    }
}