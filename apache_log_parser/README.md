# Apache Log Parser

## Challenge Description

Parse Apache web server logs from a remote URL, extract log levels (e.g., `notice`, `error`) from each line, and organize log entries into separate files based on their severity level.

The log file contains entries in the format:
```
[timestamp] [log_level] [additional info]
```

The goal is to stream the log file efficiently without loading it entirely into memory, extract the log level from each line using regex, and write each log entry to a corresponding output file.

## Solution

This solution uses:
- **Tokio async runtime** for non-blocking I/O operations
- **Reqwest** with streaming support to fetch logs from a remote URL
- **Regex** to extract log levels from the second bracketed field
- **Async file operations** to create/append to output files dynamically

### Key Features:
- Streams log data line-by-line to minimize memory usage
- Dynamically creates output files named `Apache_2k-[log_level].txt`
- Uses regex pattern `^\[.*?\]\s*\[([^\]]+)\]` to capture log levels
- Handles errors gracefully with Rust's `Result` type

## Steps to Run

### Prerequisites
- Rust (2024 edition or later)
- Internet connection (to fetch remote logs)

### Running the Challenge

1. Navigate to the challenge directory:
   ```bash
   cd apache_log_parser
   ```

2. Run the program:
   ```bash
   cargo run
   ```

3. Check the output:
   ```bash
   ls output/
   ```
   
   You should see files like:
   - `Apache_2k-[error].txt`
   - `Apache_2k-[notice].txt`
   - etc.

4. View the contents of a specific log level:
   ```bash
   cat output/Apache_2k-[error].txt
   ```

### Build for Release

For better performance:
```bash
cargo build --release
./target/release/apache_log_parser
```

## Output

The program creates an `output/` directory containing separate files for each log level found in the Apache logs. Each file contains all log entries matching that specific level.

