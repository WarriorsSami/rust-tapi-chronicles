# rust-tapi-chronicles

A collection of Rust programming challenges demonstrating various concepts including async I/O, concurrent processing, and multi-producer/consumer patterns.

## Challenges

### 1. Apache Log Parser
Async log streaming and categorization by log level from remote Apache logs.

**Key concepts:** Tokio async runtime, streaming HTTP requests, regex pattern matching, file I/O

[View details →](./apache_log_parser/README.md)

### 2. Linux Log IP Parser
IPv4 address extraction and frequency analysis from Linux system logs.

**Key concepts:** Async streaming, IP address parsing, BTreeMap for sorted statistics, regex

[View details →](./linux_log_ip_parser/README.md)

### 3. Robot Assembly MPSC
Multi-threaded robot assembly simulation using producer-consumer pattern with channels.

**Key concepts:** Crossbeam channels, fan-in pattern, graceful shutdown, atomic state management

[View details →](./robot_mpsc/README.md)

## Getting Started

### Prerequisites

You'll need Rust installed on your system. If you don't have Rust installed yet, follow the instructions below.

### Installing Rust

#### macOS/Linux

1. Open a terminal and run:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Follow the on-screen instructions (typically just press Enter to accept defaults)

3. Restart your terminal or run:
   ```bash
   source $HOME/.cargo/env
   ```

4. Verify the installation:
   ```bash
   rustc --version
   cargo --version
   ```

#### Windows

1. Download and run [rustup-init.exe](https://rustup.rs/)

2. Follow the on-screen instructions

3. Restart your terminal

4. Verify the installation:
   ```powershell
   rustc --version
   cargo --version
   ```

### Running the Challenges

Each challenge is a standalone Rust crate. Navigate to the respective directory and follow the instructions in its README.

## Requirements

- Rust 2024 edition or later
- Cargo package manager (installed automatically with Rust)
