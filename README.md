# Self-Hosted Cloud Storage

## Features
- Secure file storage
- CLI-based management
- Configurable storage locations

## Prerequisites
- Rust 1.75 or later
- Cargo package manager

## Installation
```bash
git clone https://github.com/sansoune/cloud_storage.git
cd cloud_storage
cargo build --release
```



## Usage

### Start Server
```bash
cargo run --bin brain
```

### Upload File
```bash
cargo run --bin storage-cli upload -f /path/to/file
```

### List Files
```bash
cargo run --bin storage-cli list
```

### Download File
```bash
cargo run --bin storage-cli download -n filename -o output_file
```



## Contributing
1. Fork the repository
2. Create your feature branch
3. Commit changes
4. Push to the branch
5. Create a Pull Request

## License
MIT License
