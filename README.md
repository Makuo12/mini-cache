# mini-cache

`mini-cache` is a lightweight, Redis-inspired key–value store that supports **strings**, **hashes**, and **sets**.  
It runs a simple server–client architecture and allows optional persistent backups on disk.

---

## Setup

Run the setup script to build and configure the project:

```bash
./setup.sh
```

The script will:

- Check if **Cargo** is installed.  
- Ask for:
  - The path to store backups (must exist).  
  - Your shell configuration file (e.g., `~/.bashrc`, `~/.zshrc`).  
- Build the `server` and `client` binaries.  
- Place them in a `mini_bin` folder.  
- Add `mini_bin` to your `PATH`.  

After setup, you can run:

```bash
server   # starts the mini-cache server
client   # connects to the server
```

---

## Running the Server

Start the server:

```bash
server
```

Stop the server with any of the following:

- `CTRL + C`  
- Typing `exit`  
- Typing `q`  

---

## Available Commands

### Fetch commands

| Command             | Description                                        |
|---------------------|----------------------------------------------------|
| `get <key>`         | Retrieve the value of a key.                       |
| `hget <key>`        | Retrieve all fields and values in a hash. (like `HGETALL`) |
| `smembers <key>`    | Retrieve all members of a set.                     |

---

### Change commands

| Command                       | Description                               |
|-------------------------------|-------------------------------------------|
| `set <key> <value>`           | Set the value of a key.                   |
| `hset <key> <field> <value>`  | Set the value of a field in a hash.       |
| `sadd <key> <value>`          | Add a value to a set.                     |

---

### Delete commands

| Command                      | Description                                |
|------------------------------|--------------------------------------------|
| `del <key>`                  | Delete a key and its value.                |
| `hdel <key> <field>`         | Delete a specific field from a hash.       |
| `sremove <key> <value>`      | Remove a value from a set.                 |

---

## Notes

- Keys are **strings**.  
- Hash fields are stored as **key–value pairs**.  
- Data is kept **in memory**, with optional persistent backups stored on disk.  

---

## ⚡ Example Usage

```bash
client=# set name makuo
1

client=# set age 25
1

client=# hset person name makuo age 25
1

client=# hget person
name makuo age 25 

client=# smembers person
Data not found

client=# sadd humans anita james john 
1

client=# smembers person
Data not found

client=# smembers humans
anita james john
```

---

## Persistence & Backups

When running `./setup.sh`, you will be asked for a **backup path**:

```bash
Which path do you want your backups to be stored (this path should already exist):
```

Example

```bash
Which path do you want your backups to be stored (this path should already exist): /Users/mac/Documents/test

```

- The server will automatically write in-memory data to a file in this directory.  
- On restart, the server will **reload** the most recent backup, ensuring data survives crashes or restarts.  
- A backup path **must** be provided when starting the server. Without it, the server will not run.

This behavior is controlled by the `DATA_PATH` environment variable, which is passed during the build.

---

## Configuration

To run the `server` and `client` commands directly from your terminal, you need to add the path to the compiled binaries to your shell configuration file.  

For example, if you are using **zsh**, update your shell configuration file (`~/.zshrc`):

```bash
export PATH="$PATH:/path/to/your/project/target/debug"
```

This is why **Path to shell configuration file:** is asked so that it will be automatically added using thev provided path to the file.

## Customizing Binary Names

By default, the binaries are named **server** and **client**.  
You can change these names by editing the **Cargo.toml** file:

```toml
[[bin]]
name = "client"
path = "src/bin/client.rs"

[[bin]]
name = "server"
path = "src/bin/server.rs"
```

For example, to rename `server` to `mini-server`:

```toml
[[bin]]
name = "mini-server"
path = "src/bin/server.rs"
```

After rebuilding (`cargo build --release`), you’ll get a binary named `mini-server`.  
