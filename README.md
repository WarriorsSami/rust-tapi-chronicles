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

Each challenge is a standalone Rust crate. Navigate to the respective directory and follow the instructions in its README.

## Requirements

- Rust 2024 edition or later
- Cargo package manager
