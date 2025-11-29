# Linux Log IP Parser

## Challenge Description

Parse Linux system logs from a remote source, extract all IPv4 addresses from each log line, count their occurrences, and generate a statistical report sorted by IP address.

The challenge requires:
- Extracting valid IPv4 addresses (with format validation)
- Counting frequency of each IP address across all log entries
- Producing a sorted output file with IP addresses and their occurrence counts

## Solution

This solution uses:
- **Tokio async runtime** for asynchronous streaming and file operations
- **Reqwest** with streaming to efficiently download large log files
- **Custom IPv4Address struct** with parsing logic and validation
- **BTreeMap** for automatic sorting and frequency counting
- **Regex** for IPv4 pattern matching (handles both `.` and `-` separators)

### Key Features:
- Streams log data line-by-line to handle large files efficiently
- Validates IPv4 addresses (0-255 range for each octet)
- Uses BTreeMap for automatic lexicographic sorting of IP addresses
- Outputs formatted statistics with aligned columns
- Regex pattern: `(25[0-5]|2[0-4]\d|[01]?\d?\d)[\.-](25[0-5]|2[0-4]\d|[01]?\d?\d)[\.-](25[0-5]|2[0-4]\d|[01]?\d?\d)[\.-](25[0-5]|2[0-4]\d|[01]?\d?\d)`

### Implementation Highlights:
- Custom `IPv4Address` type with `Display` trait for formatting
- `try_parse` method for safe parsing with validation
- Efficient in-memory counting using BTreeMap
- Fixed-width formatting for clean output alignment

## Steps to Run

### Prerequisites
- Rust (2024 edition or later)
- Internet connection (to fetch remote logs)

### Running the Challenge

1. Navigate to the challenge directory:
   ```bash
   cd linux_log_ip_parser
   ```

2. Run the program:
   ```bash
   cargo run
   ```

3. Check the output:
   ```bash
   cat output/Linux2k_IP_stat.txt
   ```
   
   The output will show IP addresses and their counts in a formatted table:
   ```
   10.0.0.1        5
   192.168.1.100   12
   192.168.1.101   8
   ...
   ```

### Build for Release

For better performance:
```bash
cargo build --release
./target/release/linux_log_ip_parser
```

## Output

The program creates `output/Linux2k_IP_stat.txt` containing:
- One line per unique IP address
- IP addresses sorted in ascending order
- Count of occurrences for each IP
- Fixed-width formatting (15 chars for IP, followed by count)

