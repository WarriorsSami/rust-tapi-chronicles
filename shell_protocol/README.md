# Shell Protocol - Remote File System Operations

## Challenge Overview

Implement a client-server system for remote file system operations supporting both **TCP** and **UDP** protocols. The system provides:

- Remote directory navigation (cd, cd.., dir/ls)
- File system operations (mkdir, copy)
- File transfer capabilities (upload, download)
- Client-server architecture with proper error handling
- Support for both connection-oriented (TCP) and connectionless (UDP) protocols

This challenge explores fundamental differences between TCP and UDP in the context of file operations, demonstrating how each protocol handles connection management, data transfer, and reliability.

---

## TCP Implementation

### Challenge Description

Create a **connection-oriented** client-server system using TCP that:
- Establishes persistent connections between client and server
- Implements streaming file transfers
- Manages single-client connections with rejection of concurrent attempts
- Maintains stateful sessions per TCP connection

### Solution Description

#### Key Features

- **Persistent Connection**: Client maintains continuous TCP connection throughout the session
- **Stream-Based File Transfer**: Files transferred as continuous byte streams using `std::io::copy()`
- **Single-Client Model**: Server accepts only one client at a time, rejecting additional attempts with error messages
- **Stateful Session**: Current working directory maintained per TCP connection
- **Server Persistence**: Server continues running after client disconnection
- **Binary Protocol**: Uses bincode with serde for efficient request/response serialization
- **Graceful Error Handling**: Comprehensive error messages for all operations

#### Architecture

```
┌──────────────────┐         TCP Connection           ┌──────────────────┐
│   TCP Client     │◄─────────────────────────────►   │   TCP Server     │
│                  │      (Stream-based I/O)          │                  │
├──────────────────┤                                  ├──────────────────┤
│ • Send Requests  │                                  │ • Accept Client  │
│ • Stream Upload  │                                  │ • Reject Others  │
│ • Stream Download│                                  │ • Process Reqs   │
│ • Command Loop   │                                  │ • Stream Files   │
└──────────────────┘                                  └──────────────────┘
```

**File Transfer Flow:**

**Upload:**
```
Client → Upload{file_name, size} → Server (creates file)
Client ← Ok ← Server
Client ══► Raw bytes stream ══► Server (writes continuously)
Complete
```

**Download:**
```
Client → Download{src_path} → Server (opens file)
Client ← FileMetadata{name, size} ← Server
Client ◄══ Raw bytes stream ◄══ Server (reads & streams)
Complete
```

#### Technical Details

- **Transport:** `TcpStream`, `TcpListener` with blocking I/O
- **Serialization:** Bincode for protocol messages
- **Buffer Size:** 8192 bytes for file operations
- **Connection Model:** One client blocks server, others rejected

### Steps to Run - TCP

#### 1. Start the TCP Server

```bash
cd shell_protocol
cargo run --bin shell_protocol_tcp_server 127.0.0.1:8888 ./test_root
```

**Output:** 
```
Server listening on 127.0.0.1:8888
```

**Arguments:**
- `<address:port>` - IP address and port to bind (e.g., `127.0.0.1:8888`)
- `<root_dir>` - Root directory for file operations

#### 2. Start the TCP Client

Open a new terminal:
```bash
cd shell_protocol
cargo run --bin shell_protocol_tcp_client
```

**Prompt:**
```
Server address (host:port): 127.0.0.1:8888
Connected to 127.0.0.1:8888
>
```

#### 3. Available Commands

```bash
# Directory operations
> dir                           # List current directory
> cd test_folder                # Change directory
> cd..                          # Go to parent directory
> mkdir my_folder               # Create directory

# File operations
> copy source.txt dest.txt      # Copy file on server
> upload /path/local.txt .      # Upload file to server
> download remote.txt ./        # Download file from server

# Other
> help                          # Show available commands
> exit                          # Disconnect client
```

#### 4. Testing Single-Client Behavior

**Terminal 1 - Server:**
```bash
cargo run --bin shell_protocol_tcp_server 127.0.0.1:8888 ./test_root
```

**Terminal 2 - Client 1 (Connected):**
```bash
cargo run --bin shell_protocol_tcp_client
Server address (host:port): 127.0.0.1:8888
Connected to 127.0.0.1:8888
> dir
# Client is active
```

**Terminal 3 - Client 2 (Rejected):**
```bash
cargo run --bin shell_protocol_tcp_client
Server address (host:port): 127.0.0.1:8888
Error: Server busy: another client is already connected
```

**Server Output:**
```
Client connected: 127.0.0.1:54321
Connection attempt from 127.0.0.1:54322 rejected (server busy)
Client disconnected
# Server still running, ready for next client
```

---

## UDP Implementation

### Challenge Description

Create a **connectionless** client-server system using UDP that:
- Uses datagrams for all communication without persistent connections
- Implements chunked file transfers with application-level acknowledgments
- Supports multiple simultaneous clients with independent sessions
- Manages session state based on client addresses
- Implements reliability mechanisms at the application layer

### Solution Description

#### Key Features

- **Connectionless Communication**: No persistent connection; each request is independent
- **Chunked File Transfer**: Files split into 8KB chunks with per-chunk acknowledgments
- **Multi-Client Support**: Server handles multiple clients simultaneously using event loop
- **Session Management**: Sessions tracked by client IP:Port address with 5-minute timeout
- **Application-Level Reliability**: Custom acknowledgment protocol ensures data delivery
- **Real-Time Progress**: Percentage-based progress display during file transfers
- **Automatic Session Cleanup**: Inactive sessions expire and are cleaned up automatically

#### Architecture

```
┌─────────────┐                                 ┌──────────────────────┐
│ UDP Client A│◄────UDP Datagrams (8KB)──────►  │                      │
│ :54321      │                                 │    UDP Server        │
└─────────────┘                                 │    (Port 9999)       │
                                                │                      │
┌─────────────┐                                 │  ┌────────────────┐  │
│ UDP Client B│◄────UDP Datagrams (8KB)──────►  │  │ Session Map    │  │
│ :54322      │                                 │  ├────────────────┤  │
└─────────────┘                                 │  │ 127.0.0.1:54321│  │
                                                │  │  ├─ cwd        │  │
┌─────────────┐                                 │  │  ├─ upload     │  │
│ UDP Client C│◄────UDP Datagrams (8KB)──────►  │  │  └─ download   │  │
│ :54323      │                                 │  │ 127.0.0.1:54322│  │
└─────────────┘                                 │  │  └─ state      │  │
                                                │  └────────────────┘  │
   All clients operate simultaneously!          └──────────────────────┘
```

**File Transfer Flow:**

**Upload with Chunking:**
```
Client → Upload{file_name, size} → Server (creates file)
Client ← Ok ← Server
Client → UploadChunk{id:0, data[8KB]} → Server (writes)
Client ← ChunkAck{id:0} ← Server
Client → UploadChunk{id:1, data[8KB]} → Server (writes)
Client ← ChunkAck{id:1} ← Server
...
Client → UploadChunk{id:N, last=true} → Server (flushes)
Client ← ChunkAck{id:N} ← Server
Complete
```

**Download with Chunking:**
```
Client → Download{src_path} → Server (opens file)
Client ← FileMetadata{name, size} ← Server
Client → DownloadChunk{id:0} → Server
Client ← FileChunk{id:0, data[8KB]} ← Server
Client → DownloadChunk{id:1} → Server
Client ← FileChunk{id:1, data[8KB]} ← Server
...
Client → DownloadChunk{id:N} → Server
Client ← FileChunk{id:N, last=true} ← Server
Complete
```

#### Technical Details

- **Transport:** `UdpSocket` with datagram-based communication
- **Chunk Size:** 8192 bytes (8KB)
- **Max UDP Packet:** 65,507 bytes
- **Timeout:** 5 seconds per request
- **Session Timeout:** 5 minutes of inactivity
- **Reliability:** Per-chunk acknowledgments with chunk ID verification

### Steps to Run - UDP

#### 1. Start the UDP Server

```bash
cd shell_protocol
cargo run --bin shell_protocol_udp_server 127.0.0.1:9999 ./test_root
```

**Output:**
```
UDP Server listening on 127.0.0.1:9999
```

The server logs activity as clients connect:
```
Received 45 bytes from 127.0.0.1:54321
New session from 127.0.0.1:54321
Sent 128 bytes to 127.0.0.1:54321
```

#### 2. Start UDP Client(s)

You can start **multiple clients simultaneously**!

**Terminal 2 - Client A:**
```bash
cd shell_protocol
cargo run --bin shell_protocol_udp_client
```

**Prompt:**
```
Server address (host:port): 127.0.0.1:9999
Connected to 127.0.0.1:9999
>
```

**Terminal 3 - Client B (Simultaneous!):**
```bash
cargo run --bin shell_protocol_udp_client
Server address (host:port): 127.0.0.1:9999
Connected to 127.0.0.1:9999
>
```

**Terminal 4 - Client C (Also works!):**
```bash
cargo run --bin shell_protocol_udp_client
Server address (host:port): 127.0.0.1:9999
Connected to 127.0.0.1:9999
>
```

All clients can send commands simultaneously! ✨

#### 3. Available Commands

Same commands as TCP, plus progress indicators:

```bash
# Directory operations
> dir                           # List current directory
> cd test_folder                # Change directory
> cd..                          # Go to parent directory
> mkdir my_folder               # Create directory

# File operations
> copy source.txt dest.txt      # Copy file on server

# Upload with progress
> upload /path/large.bin .
Uploading large.bin (25000 bytes)
Server ready to receive file
Uploading: 8192/25000 bytes (32.8%)
Uploading: 16384/25000 bytes (65.5%)
Uploading: 24576/25000 bytes (98.3%)
Uploading: 25000/25000 bytes (100.0%)
Upload complete: large.bin (25000 bytes)

# Download with progress
> download large.bin ./
Downloading large.bin (25000 bytes)
Downloading: 8192/25000 bytes (32.8%)
Downloading: 16384/25000 bytes (65.5%)
Downloading: 24576/25000 bytes (98.3%)
Downloading: 25000/25000 bytes (100.0%)
Download complete: large.bin (25000 bytes) → ./large.bin

# Other
> help                          # Show available commands
> exit                          # Disconnect client
```

#### 4. Testing Multi-Client Behavior

Run commands in multiple clients **simultaneously**:

**Client A:**
```bash
> mkdir client_a_folder
Ok
> upload file_a.txt .
Uploading file_a.txt (1024 bytes)
...
Upload complete
```

**Client B (at the same time):**
```bash
> mkdir client_b_folder
Ok
> upload file_b.txt .
Uploading file_b.txt (2048 bytes)
...
Upload complete
```

**Server shows both:**
```
New session from 127.0.0.1:54321
Starting upload: file_a.txt (1024 bytes)
New session from 127.0.0.1:54322
Starting upload: file_b.txt (2048 bytes)
Upload complete: ./test_root/file_a.txt (1024 bytes)
Upload complete: ./test_root/file_b.txt (2048 bytes)
```

---

## Protocol Comparison

### Key Differences Table

| Aspect | TCP Implementation | UDP Implementation |
|--------|-------------------|-------------------|
| **Connection Model** | ✅ Connection-oriented | ❌ Connectionless |
| **Client Support** | ❌ Single client only | ✅ Multiple simultaneous |
| **Concurrent Connections** | ❌ Rejected | ✅ All accepted |
| **File Transfer** | Stream-based (continuous) | Chunk-based (8KB packets) |
| **Reliability** | ✅ TCP built-in | ⚙️ Application-level ACKs |
| **Ordering** | ✅ TCP guarantees | ⚙️ Chunk ID verification |
| **Progress Display** | Basic byte count | ✅ Real-time percentage |
| **Session Management** | Per connection | Per address + timeout |
| **State Lifetime** | Until disconnect | 5-minute timeout |
| **Packet Size Limit** | ❌ No limit | ✅ 65KB per datagram |
| **Network Overhead** | Higher (TCP headers) | Lower (UDP headers) |
| **Implementation** | Lower complexity | Higher complexity |
| **Scalability** | Limited (1 client) | High (multiple clients) |
| **Latency** | Higher (handshake) | Lower (immediate) |
| **Use Case** | Single-user, reliable | Multi-user, concurrent |

### Detailed Differences

#### 1. Connection Establishment

**TCP:**
- Requires 3-way handshake (SYN, SYN-ACK, ACK)
- Connection state maintained throughout session
- Additional latency before first request
- Automatic disconnect detection

**UDP:**
- No handshake required
- Immediate request/response communication
- Lower initial latency
- Address-based sessions with manual timeout

#### 2. File Transfer Mechanism

**TCP - Streaming:**
```rust
// Simple continuous streaming
std::io::copy(&mut file, &mut stream)?;
stream.flush()?;
```
- Continuous byte stream
- TCP handles fragmentation and ordering
- No application-level chunking
- Automatic flow control

**UDP - Chunking:**
```rust
// Explicit chunking with acknowledgments
loop {
    let chunk = read_chunk(file, 8192)?;
    send_chunk(socket, chunk_id, chunk)?;
    wait_for_ack(socket, chunk_id)?;
    chunk_id += 1;
}
```
- Explicit 8KB chunks
- Per-chunk acknowledgment required
- Manual ordering verification
- Custom flow control logic

#### 3. Concurrency Model

**TCP:** One client blocks server from accepting others

**UDP:** All clients processed in event loop without blocking

#### 4. Reliability Mechanisms

**TCP:**
- Delivery guaranteed by TCP stack
- Ordering guaranteed by TCP stack
- Automatic retransmission
- Built-in flow control

**UDP:**
- No delivery guarantees
- Application implements acknowledgments
- Chunk ID verification for ordering
- 5-second timeout per request
- Manual retry logic

#### 5. Session State Management

**TCP:**
- State tied to connection lifetime
- Automatic cleanup on disconnect
- Simple memory management

**UDP:**
- Explicit session tracking in HashMap
- Manual timeout logic (5 minutes)
- Periodic cleanup required

### Use Case Recommendations

**Choose TCP when:**
- ✅ Single client or serialized access acceptable
- ✅ Built-in TCP reliability desired
- ✅ Simpler implementation preferred
- ✅ Large file streaming is efficient
- ✅ Connection state tracking important

**Choose UDP when:**
- ✅ Multiple concurrent clients required
- ✅ Lower latency more important
- ✅ Fine-grained transmission control needed
- ✅ Connectionless model fits architecture
- ✅ Can implement application reliability

---

## Building and Testing

### Build All Binaries

```bash
cd shell_protocol

# Debug mode (for development)
cargo build --bins

# Release mode (optimized)
cargo build --release --bins
```

**Binaries created:**
- `target/debug/shell_protocol_tcp_server`
- `target/debug/shell_protocol_tcp_client`
- `target/debug/shell_protocol_udp_server`
- `target/debug/shell_protocol_udp_client`

### Quick Test Scripts

**Test TCP:**
```bash
#!/bin/bash
cargo run --bin shell_protocol_tcp_server 127.0.0.1:8888 test_root &
SERVER_PID=$!
sleep 2

echo "test data" > test.txt
(echo "127.0.0.1:8888"; sleep 1; echo "upload test.txt ."; sleep 2; echo "exit") | \
  cargo run --bin shell_protocol_tcp_client

kill $SERVER_PID
```

**Test UDP:**
```bash
#!/bin/bash
cargo run --bin shell_protocol_udp_server 127.0.0.1:9999 test_root &
SERVER_PID=$!
sleep 2

# Test with multiple clients
(echo "127.0.0.1:9999"; sleep 1; echo "mkdir client1"; sleep 1; echo "exit") | \
  cargo run --bin shell_protocol_udp_client &

(echo "127.0.0.1:9999"; sleep 1; echo "mkdir client2"; sleep 1; echo "exit") | \
  cargo run --bin shell_protocol_udp_client &

wait
kill $SERVER_PID
```

---

## Project Structure

```
shell_protocol/
├── Cargo.toml                    # Dependencies and binary configurations
├── README.md                     # This file
├── src/
│   ├── lib.rs                    # Shared protocol definitions
│   │                             #   - Request/Response enums
│   │                             #   - Chunk-related messages
│   └── bin/
│       ├── tcp_server.rs         # TCP server implementation
│       ├── tcp_client.rs         # TCP client implementation
│       ├── udp_server.rs         # UDP server with session management
│       └── udp_client.rs         # UDP client with chunking
├── test_root/                    # Default server root directory
└── target/                       # Build artifacts
```

---

## Dependencies

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
bincode = { version = "2.0", features = ["serde", "derive"] }
```

- **serde:** Serialization framework
- **bincode:** Binary encoding/decoding for protocol messages

---

## Summary

This challenge demonstrates the fundamental differences between TCP and UDP protocols through practical file system operations:

### TCP Implementation
- **Model:** Connection-oriented, single-client
- **Strength:** Simple, reliable, streaming transfers
- **Trade-off:** Limited to one client at a time
- **Best for:** Single-user scenarios, reliable networks

### UDP Implementation
- **Model:** Connectionless, multi-client
- **Strength:** Concurrent clients, lower latency
- **Trade-off:** Complex reliability layer, chunking overhead
- **Best for:** Multi-user scenarios, scalable systems

Both implementations provide complete file system operation capabilities with different architectural approaches, demonstrating network protocol design trade-offs in real-world applications.
