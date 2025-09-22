use std::{collections::HashMap, io::{BufRead, BufReader, SeekFrom, Write}, path::PathBuf};
use std::fs::{File, OpenOptions};


use bytes::{BufMut, Bytes, BytesMut};
use tokio::{fs::OpenOptions as OpenOptionsTokio, io::{AsyncBufReadExt, AsyncSeekExt, AsyncWriteExt, BufReader as TokioBufReader}, sync::mpsc::Sender};

use std::fmt::{self, Display, Debug};

use crate::utils::Cache;

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
#[derive(Debug)]
pub struct Delete {
    pub cmd: Cache,
    pub key_value: String,
    pub key: String
}


impl Delete {
    pub fn update_key_value(&mut self, key_value: String) {
        self.key_value = key_value;
    }
}


pub enum Pipe {
    Recent(String), Delete(Delete)
}

#[derive(Debug, PartialEq)]
pub enum DeleteType {
    Item(String), Recent(String), None
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
        let mut buf = BytesMut::with_capacity(buffer.len()+10000);
        buf.put(buffer.as_bytes());
        Ok(Memory {path, buffer: buf, item, recent })
    }

    pub async fn recent_to_file_schedular(&mut self, key: String, value: &Bytes) {
        let mut file = match OpenOptionsTokio::new().append(true).write(true).open(&self.path).await {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error at reading in file {}", e);
                return;
            }
        };
        let mut data = String::new();
        if value.is_empty() {
            return;
        }
        for b in value.slice(..) {
            data.push(b as char);
        }
        self.item.insert(key, Position { start: self.buffer.len(), 
            end: self.buffer.len() + value.len()});
        self.buffer.put(&value[..]);
        data.push('\n');
        let _ = match file.write(&data.as_bytes()).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error at: {}", e);
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
                eprintln!("Error at: {}", e);
                return;
            }
        };
        
    }
    pub async fn get(&self, mut key_value: String) -> CacheResult {
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
    pub async fn del(&mut self, delete: Delete, tx: Sender<Pipe>) -> CacheResult {
        if delete.key_value.trim().is_empty() {
            return CacheResult::Failure(delete.key_value+"Key cannot be empty");
        } else {
            return self.handle_del(delete, tx).await;
        }
    }
    fn handle_text_sm(key: &String, result: &[u8]) -> String {
        let mut text = String::new();
        let mut word = String::new();
        let mut found = false;
        for item in result {
            let c = *item as char;
            if found {
                if c == '\"' {
                    // means word is complete
                    if word != *key {
                        text.push_str(&word);
                    }
                    text.push('\"');
                    word.clear();
                } else {
                    word.push(c);
                }
            } else {
                if c == '\'' {
                    // We use word to file the word
                    found = true;
                    text.push(c);
                } else {
                    text.push(c);
                }
            }
        }
        text
    }
    fn handle_text(key: &String, result: &[u8]) -> String {
        let mut text = String::new();
        let mut word = String::new();
        let mut word_count = 0;
        let mut found = false;
        for item in result {
            let c = *item as char;
            if found {
                if c == '\"' {
                    // means word is complete
                    if word_count > 0 {
                        word_count = 0;
                        word.clear();
                        continue;
                    }
                    if word != *key {
                        text.push_str(&word);
                    } else {
                        word_count+=1;
                    }
                    text.push('\"');
                    word.clear();
                } else {
                    if word_count == 0 {
                        word.push(c);
                    }
                }
            } else {
                if c == '\'' {
                    // We use word to file the word
                    found = true;
                    text.push(c);
                } else {
                    text.push(c);
                }
            }
        }
        text
    }
    pub async fn handle_del(&mut self, mut del: Delete, tx: Sender<Pipe>) -> CacheResult {
        match del.cmd {
            Cache::Del => {
                let mut delete_type = DeleteType::None;
                for item in self.recent.keys() {
                    let keys: Vec<&str> = item.split('\t').collect();
                    if keys.len() != 2 {
                        return CacheResult::Failure("Item not found".to_string());
                    }
                    if keys[1] == del.key {
                        delete_type = DeleteType::Recent(item.clone());
                    }
                }
                if delete_type == DeleteType::None {
                    for item in self.item.keys() {
                        let keys: Vec<&str> = item.split('\t').collect();
                        if keys.len() != 2 {
                            return CacheResult::Failure("Item not found".to_string());
                        }
                        if keys[1] == del.key {
                            delete_type = DeleteType::Item(item.clone());
                        }
                    }
                }
                match delete_type {
                    DeleteType::Item(value) => {
                        let result = self.item.remove(&value);
                        if result.is_none() {
                            return CacheResult::Failure("Item not found".to_string());
                        }
                        del.update_key_value(value);
                        let _ = tx.send(Pipe::Delete(del)).await;
                        return CacheResult::Success("1".to_string())
                    },
                    DeleteType::Recent(value) => {
                        let result = self.recent.remove(&value);
                        if result.is_none() {
                            return CacheResult::Failure("Item not found".to_string());
                        }
                        del.update_key_value(value);
                        let _ = tx.send(Pipe::Delete(del)).await;
                        return CacheResult::Success("1".to_string())
                    }, 
                    DeleteType::None => {
                        return CacheResult::Failure("Item not found".to_string());
                    }
                }
            },
            Cache::HDel | Cache::SRemove => {
                if let Some(result ) = self.recent.get(&del.key_value) {
                    let text = match del.cmd {
                        Cache::HDel => Memory::handle_text(&del.key, result),
                        Cache::SRemove => Memory::handle_text_sm(&del.key, result),
                        _ => String::new()
                    };
                    if text.is_empty() {
                        return CacheResult::Failure("Not item found".to_string());
                    } else {
                        self.recent.insert(del.key_value.to_string(), Bytes::from(text));
                        let _ = tx.send(Pipe::Delete(del)).await;
                        return CacheResult::Success("1".to_string());
                    }
                } else {
                    if let Some(result) = self.item.get(&del.key_value) {
                        // let mut main_buffer = BytesMut::new();
                        let items = &self.buffer[result.start..result.end];
                        let text = match del.cmd {
                            Cache::HDel => Memory::handle_text(&del.key, items),
                            Cache::SRemove => Memory::handle_text_sm(&del.key, items),
                            _ => String::new()
                        };
                        if text.is_empty() {
                            return CacheResult::Failure("Not item found".to_string());
                        } else {
                            let mut num = 0;
                            while num < text.as_bytes().len() {
                                let value = text.as_bytes()[num];
                                self.buffer[result.start+num] = value;
                                num+=1; 
                            }
                            let position = Position {start: result.start, end: result.start+text.as_bytes().len()};
                            self.item.insert(del.key_value.clone(), position);
                            let _ = tx.send(Pipe::Delete(del)).await;
                            return CacheResult::Success("1".to_string());
                        }
                    } else {
                        return CacheResult::Failure("Not item found".to_string());
                    }
                }
            }, 
            _ => return CacheResult::Failure("Not item found".to_string())
        }
    }
    pub async fn modify_file(&self, del: Delete) -> Result<(), std::io::Error> {
        let file = match OpenOptionsTokio::new().read(true).write(true).open(&self.path).await {
            Ok(f) => f,
            Err(e) => panic!("Cannot read line {}", e)
        };
        let reader_file = match file.try_clone().await {
            Ok(value) => value,
            Err(e) => panic!("error at clone {}", e)
        };
        let mut writer_file = file;
        let mut new_file: String = String::new();
        let reader = TokioBufReader::new(reader_file);
        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            let mut line_data = line.split('\'');
            let command_key = line_data.next().unwrap_or("");

            let mut found = false;

            if command_key == del.key_value {
                match del.cmd {
                    Cache::Del => {
                        found = true;
                        if let Some(next_line) = lines.next_line().await? {
                            new_file.push_str(&next_line);
                            new_file.push('\n');
                        }
                    }
                    Cache::HDel => {
                        let mut text = String::new();
                        let split: Vec<&str> = line.split('\'').collect();
                        text.push_str(&split[0]);

                        if split.len() > 1 {
                            text.push('\'');
                            let items: Vec<&str> = split[1].split('"').collect();
                            let mut i = 0;
                            while i < items.len() {
                                if items[i] == del.key {
                                    i += 2;
                                    continue;
                                }
                                text.push_str(items[i]);
                                text.push('"');
                                i += 1;
                            }
                        }

                        new_file.push_str(&text);
                        new_file.push('\n');
                        found = true;
                    }
                    Cache::SRemove => {
                        let mut text = String::new();
                        let split: Vec<&str> = line.split('\'').collect();
                        text.push_str(&split[0]);

                        if split.len() > 1 {
                            text.push('\'');
                            let items: Vec<&str> = split[1].split('"').collect();
                            let mut i = 0;
                            while i < items.len() {
                                if items[i] == del.key {
                                    i += 1;
                                    continue;
                                }
                                text.push_str(items[i]);
                                text.push('"');
                                i += 1;
                            }
                        }

                        new_file.push_str(&text);
                        new_file.push('\n');
                        found = true;
                    }
                    _ => continue,
                }
            }
            if !found {
                new_file.push_str(&line);
                new_file.push('\n');
            }
        }
        writer_file.set_len(0).await?;
        writer_file.seek(SeekFrom::Start(0)).await?;
        let _ = match writer_file.write_all(new_file.as_bytes()).await {
            Ok(value) => value,
            Err(e) => panic!("error at write {e}")
        };
        return Ok(())
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
    pub async fn set(&mut self, key: String, value: String, mut action: String, tx: Sender<Pipe>) -> CacheResult {
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
        self.recent.insert(action.clone(), Bytes::from(value));
        let _ = tx.send(Pipe::Recent(action)).await;
        return CacheResult::Success(String::from("1"));
    }
}

#[derive(PartialEq)]
enum MemoryType {
    Command,
    Value
}

impl Drop for Memory {
    fn drop(&mut self) {
        self.recent_to_file();
    }
}