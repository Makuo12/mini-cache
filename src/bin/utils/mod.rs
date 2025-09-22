use core::str;
use std::sync::Arc;

use tokio::sync::{mpsc::Sender, Mutex};

pub mod models;
pub mod file_control;

use models::{MainError, Memory};

use crate::utils::models::{Delete, Pipe};

pub const FETCH_CMD: [&'static str; 3] = ["get", "hget", "smembers"];
pub const CHANGE_CMD: [&'static str; 3] = ["set", "hset", "sadd"];
pub const DEL_CMD: [&'static str; 3] = ["del", "hdel", "sremove"];

#[derive(Debug)]
pub enum Cache {
    Set,
    HSet,
    SAdd,
    // FETCH_CMD
    Get,
    HGet,
    SMembers,

    // Delete CMD
    Del,
    HDel,
    SRemove
}

impl Cache {
    pub fn new(cmd: &Command) -> Result<Cache, MainError> {
        match &cmd.action.to_lowercase() {
            key if key == FETCH_CMD[0] => Ok(Self::Get),
            key if key == FETCH_CMD[1] => Ok(Self::HGet),
            key if key == FETCH_CMD[2] => Ok(Self::SMembers),
            key if key == CHANGE_CMD[0] => Ok(Self::Set),
            key if key == CHANGE_CMD[1] => Ok(Self::HSet),
            key if key == CHANGE_CMD[2] => Ok(Self::SAdd),
            key if key == DEL_CMD[0] => Ok(Self::Del),
            key if key == DEL_CMD[1] => Ok(Self::HDel),
            key if key == DEL_CMD[2] => Ok(Self::SRemove),
            _ => Err(MainError::FindCacheTypeError(String::from("Cache not found")))
        }
    }
    pub async fn handle_cmd(&self, cmd: Command, memory: Arc<Mutex<Memory>>, tx: Sender<Pipe>) -> CacheResult {
        match self {
            Self::Set => {
                if cmd.len() == 2 {
                    return self.set(cmd, memory, tx).await;
                }
                CacheResult::Failure(String::from("Use set to store a single key an value pair.\nset key value"));
            }
            Self::HSet => {
                if cmd.len() % 2 == 1 && cmd.len() > 1 {
                    return self.set(cmd, memory, tx).await;
                }
                CacheResult::Failure(String::from("Use set to store a single key an value pair.\nset key value"));
            }
            Self::SAdd => {
                if cmd.len() > 0 {
                    return self.set(cmd, memory, tx).await;
                } 
                CacheResult::Failure(String::from("Use sdd to store 1 or more unqiue values.\nsadd key value_one value_two"));
            }
            // FETCH_CMD 
            Self::Get | Self::HGet | Self::SMembers => {
                return self.get(cmd, memory).await;
            },
            Self::Del => {
                return self.del(cmd, Cache::Del, memory, tx).await;
            },
            Self::HDel => {
                return self.del(cmd, Cache::HDel, memory, tx).await;
            },
            Self::SRemove => {
                return self.del(cmd, Cache::SRemove, memory, tx).await;
            }
        }
        return CacheResult::Failure("No action was perform.\tAre you using the right formatting".to_string());
    }
    async fn get(&self, mut cmd: Command, memory: Arc<Mutex<Memory>>) -> CacheResult {
        // key -> command\tkey
        cmd.reverse_action();
        let key_value = cmd.reverse+"\t"+&cmd.key; // We use naming key_value because this is where we would store the value
        let memory = memory.lock().await;
        memory.get(key_value).await
    }
    async fn del(&self, mut cmd: Command, cache: Cache, memory: Arc<Mutex<Memory>>, tx: Sender<Pipe>) -> CacheResult {
        cmd.reverse_action();
        let key_value = cmd.reverse+"\t"+&cmd.key; // We use naming key_value because this is where we would store the value
        let delete = Delete{cmd: cache, key_value, key: cmd.del_action};
        let mut memory = memory.lock().await;
        memory.del(delete, tx).await
    }
    async fn set(&self, cmd: Command, memory: Arc<Mutex<Memory>>, tx: Sender<Pipe>) -> CacheResult {
        // key -> command\tkey
        // value -> value\"value\"value\n
        let mut memory = memory.lock().await;
        return memory.set(cmd.key, cmd.data, cmd.action, tx).await;
    }

}

pub struct Command{
    data: String,
    key: String,
    action: String, 
    del_action: String,
    reverse: String
}

impl Command {
    pub fn new(size: usize, data: Vec<u8>) -> Result<Command, MainError> {
        if data.len() < 4 || size == 0  || size > data.len() {
            return Err(MainError::BadCommandFormat(String::from("Not enough commands")));
        } 

        let mut i = 0;
        let mut key = String::new();
        let mut action = String::new();
        let mut values = String::new();
        let length = data[..size-1].len();
        let mut last = "";
        for val in data[..size-1].split(|b| *b == b'\t') {
            let val = match str::from_utf8(val) {
                Ok(v) => v,
                Err(e) => {
                    return Err(MainError::BadCommandFormat(e.to_string()))
                }
            };
            last = val;
            if i == 0 {
                i += 1;
                continue;
            }else if i == 1 {
                action+=val;
                values+=val;
            } else if i == 2 {
                key+= val;
                values+="\t";
                values+=val;
                values+="\'";
            } else if i == length - 1 {
                continue;
            } else {
                values+=val;
                values+="\"";
            }
            i += 1
        }
        return Ok(Command { data: values, key, action, del_action: last.to_string(), reverse: String::new() })
    }
    fn len(&self) -> usize {
        let mut control = self.data.split("\'");
        control.next();
        if let Some(value) = control.next() {
            if !value.contains('\"') {
                return 1;
            } else {
                return value.split('\"').count();
            }
        }
        0
    } 
    pub fn reverse_action(&mut self) {
        // pub const FETCH_CMD: [&'static str; 3] = ["get", "hget", "smembers"];
        // pub const CHANGE_CMD: [&'static str; 3] = ["set", "hset", "sadd"];
        self.reverse.clear();
        match self.action.to_lowercase() {
            key if key == FETCH_CMD[0] => self.reverse+=CHANGE_CMD[0],
            key if key == FETCH_CMD[1] => self.reverse+=CHANGE_CMD[1],
            key if key == FETCH_CMD[2] => self.reverse+=CHANGE_CMD[2],
            key if key == CHANGE_CMD[0] => self.reverse+=FETCH_CMD[0],
            key if key == CHANGE_CMD[1] => self.reverse+=FETCH_CMD[1],
            key if key == CHANGE_CMD[2] => self.reverse+=FETCH_CMD[2],
            key if key == DEL_CMD[1] => self.reverse+=CHANGE_CMD[1],
            key if key == DEL_CMD[2] => self.reverse+=CHANGE_CMD[2],
            _ => {
                self.reverse.clear()
            }
        }
    }
}

pub enum CacheResult {
    Success(String),
    Failure(String),
}