
install linker: 

```bash
# on ubuntu: 
sudo apt install lld clang

# on windows: 
cargo install -f cargo-binutils
rustup component add llvm-tools-preview

# on macos: 
brew install michaeleisel/zld/zld`
```