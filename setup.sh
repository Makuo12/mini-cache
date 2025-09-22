#! /bin/sh

# We need to know where you want to store your data
echo "Welcome to mini-cache."
echo "Data is kept in memory, with a persistent backup stored on disk."

# Check if cargo exists
if command -v cargo > /dev/null; then
    echo "✅ Cargo is already installed."
    read -p "Which path do you want your backups to be stored (this path should already exist): " path
    read -p "Path to shell configuration file: " shell
    DATA_PATH=$path cargo build -q --bin server --release
    cargo build -q --bin client --release
    if [ -d "./mini_bin" ]; then
        echo "bin setup"
    else
        mkdir mini_bin
    fi
    mv ./target/release/server mini_bin
    mv ./target/release/client mini_bin
    cur_dir=$(pwd)
    path_exist="$(grep '.*mini_bin.*' $shell)"
    if [ -n "$path_exist" ]; then
        echo "path added"
    else
        echo "setting up path" && echo 'export PATH="$PATH:'$cur_dir'/mini_bin"' >> $shell
    fi
else
    read -p "⚠️ Cargo is not installed. Would you like to install it now? (Y/N): " response
    if [ "$response" = "Y" ]; then
        os="$(uname -s)"
        case "$os" in
            Linux*)   
                echo "Please install Cargo using your package manager or Rustup on Linux, then run setup again."
                ;;
            Darwin*)  
                echo "You can install Cargo on macOS by running: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
                ;;
            *)        
                echo "Unknown operating system: $os. Please install Cargo manually from https://www.rust-lang.org/tools/install"
                ;;
        esac
    else
        echo "Cargo is required to continue. Exiting setup."
    fi
fi
