use bincode::config::standard;
use bincode::{decode_from_slice, encode_to_vec};
use shell_protocol::{Request, Response};
use std::fs::File;
use std::io::{self, BufRead, Read, Write};
use std::net::UdpSocket;
use std::time::Duration;

const MAX_PACKET_SIZE: usize = 65507;
const TIMEOUT_SECS: u64 = 5;
const CHUNK_SIZE: usize = 8192;

fn send_request(socket: &UdpSocket, req: &Request) -> io::Result<Response> {
    // Encode request
    let data = encode_to_vec(req, standard())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("encode error: {e}")))?;

    // Send request
    socket.send(&data)?;

    // Receive response
    let mut buf = vec![0u8; MAX_PACKET_SIZE];
    let size = socket.recv(&mut buf)?;

    // Decode response
    let (resp, _): (Response, _) = decode_from_slice(&buf[..size], standard())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("decode error: {e}")))?;

    Ok(resp)
}

fn print_dir_list(entries: &[shell_protocol::DirEntry]) {
    for entry in entries {
        if entry.is_dir {
            println!("{}/", entry.name);
        } else {
            println!("{}", entry.name);
        }
    }
}

fn do_upload(socket: &UdpSocket, local_path: &str, remote_folder: &str) -> io::Result<()> {
    let mut f = File::open(local_path)?;
    let metadata = f.metadata()?;
    let size = metadata.len();
    let filename = std::path::Path::new(local_path)
        .file_name()
        .and_then(|os| os.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid filename"))?
        .to_string();

    println!("Uploading {} ({} bytes)", filename, size);

    // Send upload initiation request
    let req = Request::Upload {
        dst_path: remote_folder.to_string(),
        file_name: filename.clone(),
        size,
    };

    let resp = send_request(socket, &req)?;
    match resp {
        Response::Ok => {
            println!("Server ready to receive file");
        }
        Response::Error(msg) => {
            eprintln!("Upload error: {}", msg);
            return Err(io::Error::new(io::ErrorKind::Other, msg));
        }
        _ => {
            eprintln!("Unexpected response: {:?}", resp);
            return Err(io::Error::new(io::ErrorKind::Other, "Unexpected response"));
        }
    }

    // Send file in chunks
    let mut chunk_id = 0u32;
    let mut total_sent = 0u64;
    let mut buf = vec![0u8; CHUNK_SIZE];

    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }

        let is_last = n < CHUNK_SIZE;
        let chunk_data = buf[..n].to_vec();

        let chunk_req = Request::UploadChunk {
            chunk_id,
            data: chunk_data,
            is_last,
        };

        let chunk_resp = send_request(socket, &chunk_req)?;
        match chunk_resp {
            Response::ChunkAck { chunk_id: ack_id } => {
                if ack_id != chunk_id {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Chunk ID mismatch: expected {}, got {}", chunk_id, ack_id),
                    ));
                }
                total_sent += n as u64;
                print!(
                    "\rUploading: {}/{} bytes ({:.1}%)",
                    total_sent,
                    size,
                    (total_sent as f64 / size as f64) * 100.0
                );
                io::stdout().flush()?;

                if is_last {
                    println!();
                    println!("Upload complete: {} ({} bytes)", filename, total_sent);
                    break;
                }
            }
            Response::Error(msg) => {
                eprintln!("\nUpload error: {}", msg);
                return Err(io::Error::new(io::ErrorKind::Other, msg));
            }
            _ => {
                eprintln!("\nUnexpected response: {:?}", chunk_resp);
                return Err(io::Error::new(io::ErrorKind::Other, "Unexpected response"));
            }
        }

        chunk_id += 1;
    }

    Ok(())
}

fn do_download(socket: &UdpSocket, remote_path: &str, local_folder: &str) -> io::Result<()> {
    // Send download request
    let req = Request::Download {
        src_path: remote_path.to_string(),
    };

    let resp = send_request(socket, &req)?;

    let (file_name, file_size) = match resp {
        Response::FileMetadata { name, size } => {
            println!("Downloading {} ({} bytes)", name, size);
            (name, size)
        }
        Response::Error(msg) => {
            eprintln!("Download error: {}", msg);
            return Err(io::Error::new(io::ErrorKind::Other, msg));
        }
        _ => {
            eprintln!("Unexpected response: {:?}", resp);
            return Err(io::Error::new(io::ErrorKind::Other, "Unexpected response"));
        }
    };

    // Create local file
    let local_path = std::path::Path::new(local_folder).join(&file_name);
    if let Some(parent) = local_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut f = File::create(&local_path)?;

    // Download file in chunks
    let mut chunk_id = 0u32;
    let mut total_received = 0u64;

    loop {
        let chunk_req = Request::DownloadChunk { chunk_id };
        let chunk_resp = send_request(socket, &chunk_req)?;

        match chunk_resp {
            Response::FileChunk {
                chunk_id: resp_chunk_id,
                data,
                is_last,
            } => {
                if resp_chunk_id != chunk_id {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "Chunk ID mismatch: expected {}, got {}",
                            chunk_id, resp_chunk_id
                        ),
                    ));
                }

                f.write_all(&data)?;
                total_received += data.len() as u64;

                print!(
                    "\rDownloading: {}/{} bytes ({:.1}%)",
                    total_received,
                    file_size,
                    (total_received as f64 / file_size as f64) * 100.0
                );
                io::stdout().flush()?;

                if is_last {
                    println!();
                    f.flush()?;
                    println!(
                        "Download complete: {} ({} bytes) → {}",
                        file_name,
                        total_received,
                        local_path.display()
                    );
                    break;
                }
            }
            Response::Error(msg) => {
                eprintln!("\nDownload error: {}", msg);
                return Err(io::Error::new(io::ErrorKind::Other, msg));
            }
            _ => {
                eprintln!("\nUnexpected response: {:?}", chunk_resp);
                return Err(io::Error::new(io::ErrorKind::Other, "Unexpected response"));
            }
        }

        chunk_id += 1;
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    print!("Server address (host:port): ");
    io::stdout().flush()?;

    let server_addr = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No input"))??;

    // Bind to any available local port
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect(&server_addr)?;
    socket.set_read_timeout(Some(Duration::from_secs(TIMEOUT_SECS)))?;

    println!("Connected to {}", server_addr);

    loop {
        print!("> ");
        io::stdout().flush()?;

        let line = match lines.next() {
            Some(Ok(l)) => l,
            Some(Err(e)) => {
                eprintln!("Read error: {}", e);
                break;
            }
            None => break,
        };

        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let cmd = parts[0];

        match cmd {
            "exit" | "quit" => {
                println!("Exiting.");
                break;
            }
            "dir" | "ls" => {
                let req = Request::Dir;
                match send_request(&socket, &req) {
                    Ok(Response::DirList(entries)) => {
                        print_dir_list(&entries);
                    }
                    Ok(Response::Error(msg)) => {
                        eprintln!("Error: {}", msg);
                    }
                    Ok(other) => {
                        eprintln!("Unexpected response: {:?}", other);
                    }
                    Err(e) => {
                        eprintln!("Request failed: {}", e);
                    }
                }
            }
            "cd" => {
                if parts.len() < 2 {
                    eprintln!("Usage: cd <path>");
                    continue;
                }
                let path = parts[1].to_string();
                let req = Request::Cd { path };
                match send_request(&socket, &req) {
                    Ok(Response::Ok) => {
                        println!("Ok");
                    }
                    Ok(Response::Error(msg)) => {
                        eprintln!("Error: {}", msg);
                    }
                    Ok(other) => {
                        eprintln!("Unexpected response: {:?}", other);
                    }
                    Err(e) => {
                        eprintln!("Request failed: {}", e);
                    }
                }
            }
            "cd.." | "cdup" => {
                let req = Request::CdUp;
                match send_request(&socket, &req) {
                    Ok(Response::Ok) => {
                        println!("Ok");
                    }
                    Ok(Response::Error(msg)) => {
                        eprintln!("Error: {}", msg);
                    }
                    Ok(other) => {
                        eprintln!("Unexpected response: {:?}", other);
                    }
                    Err(e) => {
                        eprintln!("Request failed: {}", e);
                    }
                }
            }
            "mkdir" => {
                if parts.len() < 2 {
                    eprintln!("Usage: mkdir <name>");
                    continue;
                }
                let name = parts[1].to_string();
                let req = Request::Mkdir { name };
                match send_request(&socket, &req) {
                    Ok(Response::Ok) => {
                        println!("Ok");
                    }
                    Ok(Response::Error(msg)) => {
                        eprintln!("Error: {}", msg);
                    }
                    Ok(other) => {
                        eprintln!("Unexpected response: {:?}", other);
                    }
                    Err(e) => {
                        eprintln!("Request failed: {}", e);
                    }
                }
            }
            "copy" => {
                if parts.len() < 3 {
                    eprintln!("Usage: copy <src> <dst>");
                    continue;
                }
                let src = parts[1].to_string();
                let dst = parts[2].to_string();
                let req = Request::Copy { src, dst };
                match send_request(&socket, &req) {
                    Ok(Response::CopyResult { bytes_copied }) => {
                        println!("Copied {} bytes", bytes_copied);
                    }
                    Ok(Response::Error(msg)) => {
                        eprintln!("Error: {}", msg);
                    }
                    Ok(other) => {
                        eprintln!("Unexpected response: {:?}", other);
                    }
                    Err(e) => {
                        eprintln!("Request failed: {}", e);
                    }
                }
            }
            "upload" => {
                if parts.len() < 2 {
                    eprintln!("Usage: upload <local_file> [remote_folder]");
                    continue;
                }
                let local_file = parts[1];
                let remote_folder = if parts.len() >= 3 { parts[2] } else { "." };

                match do_upload(&socket, local_file, remote_folder) {
                    Ok(_) => {}
                    Err(e) => eprintln!("Upload failed: {}", e),
                }
            }
            "download" => {
                if parts.len() < 2 {
                    eprintln!("Usage: download <remote_file> [local_folder]");
                    continue;
                }
                let remote_file = parts[1];
                let local_folder = if parts.len() >= 3 { parts[2] } else { "." };

                match do_download(&socket, remote_file, local_folder) {
                    Ok(_) => {}
                    Err(e) => eprintln!("Download failed: {}", e),
                }
            }
            "help" => {
                println!("Available commands:");
                println!("  dir / ls                          - List current directory");
                println!("  cd <path>                         - Change directory");
                println!("  cd.. / cdup                       - Go to parent directory");
                println!("  mkdir <name>                      - Create directory");
                println!("  copy <src> <dst>                  - Copy file");
                println!("  upload <local_file> [remote_dir]  - Upload file to server");
                println!("  download <remote_file> [local_dir] - Download file from server");
                println!("  help                              - Show this help");
                println!("  exit / quit                       - Exit client");
            }
            _ => {
                eprintln!("Unknown command: {}", cmd);
                eprintln!("Type 'help' for available commands");
            }
        }
    }

    Ok(())
}
