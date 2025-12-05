use bincode::config::standard;
use bincode::{decode_from_slice, encode_to_vec};
use shell_protocol::{DirEntry, Request, Response};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::UdpSocket;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_PACKET_SIZE: usize = 65507; // Maximum UDP packet size
const MAX_PAYLOAD_SIZE: usize = 65000; // Leave room for headers
const CHUNK_SIZE: usize = 8192; // Size of file chunks for transfer

#[derive(Debug)]
struct ClientSession {
    cwd: PathBuf,
    last_activity: u64,
    upload_file: Option<UploadState>,
    download_file: Option<DownloadState>,
}

#[derive(Debug)]
struct UploadState {
    file: File,
    file_path: PathBuf,
    expected_size: u64,
    received_bytes: u64,
}

#[derive(Debug)]
struct DownloadState {
    file: File,
    file_name: String,
    file_size: u64,
    sent_chunks: u32,
}

fn get_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn handle_fs_request(cwd: &mut PathBuf, root: &PathBuf, req: Request) -> Response {
    match req {
        Request::Dir => match fs::read_dir(&cwd) {
            Ok(entries) => {
                let mut list = Vec::new();
                for e in entries.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    list.push(DirEntry { name, is_dir });
                }
                Response::DirList(list)
            }
            Err(e) => Response::Error(format!("read_dir failed: {}", e)),
        },
        Request::CdUp => {
            if let Some(parent) = cwd.parent().map(|p| p.to_path_buf()) {
                if parent.starts_with(&root) {
                    *cwd = parent;
                    Response::Ok
                } else {
                    Response::Error("Cannot go above root".into())
                }
            } else {
                Response::Error("No parent".into())
            }
        }
        Request::Cd { path } => {
            let new = cwd.join(path);
            if new.is_dir() && new.starts_with(&root) {
                *cwd = new;
                Response::Ok
            } else {
                Response::Error("Invalid path or not a directory".into())
            }
        }
        Request::Mkdir { name } => {
            let new = cwd.join(name);
            match fs::create_dir(&new) {
                Ok(_) => Response::Ok,
                Err(e) => Response::Error(format!("mkdir failed: {}", e)),
            }
        }
        Request::Copy { src, dst } => {
            let src_p = cwd.join(src);
            let dst_p = cwd.join(dst);
            match fs::copy(&src_p, &dst_p) {
                Ok(bytes) => Response::CopyResult {
                    bytes_copied: bytes,
                },
                Err(e) => Response::Error(format!("copy failed: {}", e)),
            }
        }
        Request::Upload {
            dst_path,
            file_name,
            size,
        } => {
            // These are handled separately, should not reach here
            Response::Error("Upload should use UploadChunk messages".into())
        }
        Request::Download { src_path } => {
            // These are handled separately, should not reach here
            Response::Error("Download should use DownloadChunk messages".into())
        }
        Request::UploadChunk { .. } => {
            Response::Error("UploadChunk should be handled in main loop".into())
        }
        Request::DownloadChunk { .. } => {
            Response::Error("DownloadChunk should be handled in main loop".into())
        }
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: udp_server <addr:port> <root_dir>");
        std::process::exit(1);
    }
    let addr = &args[1];
    let root = PathBuf::from(&args[2]);

    let socket = UdpSocket::bind(addr)?;
    println!("UDP Server listening on {}", addr);

    // Session management: client_addr -> session
    let mut sessions: HashMap<String, ClientSession> = HashMap::new();
    let mut buf = vec![0u8; MAX_PACKET_SIZE];

    loop {
        // Clean up old sessions (inactive for > 5 minutes)
        let now = get_timestamp();
        sessions.retain(|_, session| now - session.last_activity < 300);

        match socket.recv_from(&mut buf) {
            Ok((size, src_addr)) => {
                println!("Received {} bytes from {}", size, src_addr);

                let client_key = src_addr.to_string();

                // Decode request
                let req: Request = match decode_from_slice(&buf[..size], standard()) {
                    Ok((req, _)) => req,
                    Err(e) => {
                        eprintln!("Decode error: {}", e);
                        let resp = Response::Error(format!("Invalid request: {}", e));
                        if let Ok(data) = encode_to_vec(&resp, standard()) {
                            let _ = socket.send_to(&data, src_addr);
                        }
                        continue;
                    }
                };

                // Get or create session
                let session = sessions.entry(client_key.clone()).or_insert_with(|| {
                    println!("New session from {}", src_addr);
                    ClientSession {
                        cwd: root.clone(),
                        last_activity: now,
                        upload_file: None,
                        download_file: None,
                    }
                });

                session.last_activity = now;

                // Handle request
                let resp = match req {
                    Request::Upload {
                        dst_path,
                        file_name,
                        size,
                    } => {
                        // Initialize upload
                        let dest = if dst_path == "." || dst_path.is_empty() {
                            session.cwd.join(&file_name)
                        } else {
                            session.cwd.join(&dst_path).join(&file_name)
                        };

                        if let Some(parent) = dest.parent() {
                            let _ = fs::create_dir_all(parent);
                        }

                        match File::create(&dest) {
                            Ok(file) => {
                                println!("Starting upload: {} ({} bytes)", file_name, size);
                                session.upload_file = Some(UploadState {
                                    file,
                                    file_path: dest,
                                    expected_size: size,
                                    received_bytes: 0,
                                });
                                Response::Ok
                            }
                            Err(e) => Response::Error(format!("Cannot create file: {}", e)),
                        }
                    }

                    Request::UploadChunk {
                        chunk_id,
                        data,
                        is_last,
                    } => {
                        if let Some(ref mut upload) = session.upload_file {
                            match upload.file.write_all(&data) {
                                Ok(_) => {
                                    upload.received_bytes += data.len() as u64;
                                    println!(
                                        "Received chunk {} ({} bytes, total: {}/{})",
                                        chunk_id,
                                        data.len(),
                                        upload.received_bytes,
                                        upload.expected_size
                                    );

                                    if is_last {
                                        let _ = upload.file.flush();
                                        println!(
                                            "Upload complete: {} ({} bytes)",
                                            upload.file_path.display(),
                                            upload.received_bytes
                                        );
                                        session.upload_file = None;
                                    }

                                    Response::ChunkAck { chunk_id }
                                }
                                Err(e) => {
                                    eprintln!("Write error: {}", e);
                                    session.upload_file = None;
                                    Response::Error(format!("Write error: {}", e))
                                }
                            }
                        } else {
                            Response::Error("No active upload session".to_string())
                        }
                    }

                    Request::Download { src_path } => {
                        let full = session.cwd.join(&src_path);
                        match File::open(&full) {
                            Ok(file) => match file.metadata() {
                                Ok(metadata) => {
                                    let size = metadata.len();
                                    let name = full
                                        .file_name()
                                        .and_then(|os| os.to_str())
                                        .unwrap_or("file")
                                        .to_string();

                                    println!("Starting download: {} ({} bytes)", name, size);
                                    session.download_file = Some(DownloadState {
                                        file,
                                        file_name: name.clone(),
                                        file_size: size,
                                        sent_chunks: 0,
                                    });

                                    Response::FileMetadata { name, size }
                                }
                                Err(e) => Response::Error(format!("Metadata error: {}", e)),
                            },
                            Err(e) => Response::Error(format!("Open failed: {}", e)),
                        }
                    }

                    Request::DownloadChunk { chunk_id } => {
                        if let Some(ref mut download) = session.download_file {
                            let mut buf = vec![0u8; CHUNK_SIZE];
                            match download.file.read(&mut buf) {
                                Ok(n) => {
                                    buf.truncate(n);
                                    let is_last = n < CHUNK_SIZE;

                                    println!(
                                        "Sending chunk {} ({} bytes, last: {})",
                                        chunk_id, n, is_last
                                    );

                                    download.sent_chunks += 1;

                                    if is_last {
                                        println!(
                                            "Download complete: {} ({} chunks)",
                                            download.file_name, download.sent_chunks
                                        );
                                        session.download_file = None;
                                    }

                                    Response::FileChunk {
                                        chunk_id,
                                        data: buf,
                                        is_last,
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Read error: {}", e);
                                    session.download_file = None;
                                    Response::Error(format!("Read error: {}", e))
                                }
                            }
                        } else {
                            Response::Error("No active download session".to_string())
                        }
                    }

                    other => handle_fs_request(&mut session.cwd, &root, other),
                };

                // Encode and send response
                match encode_to_vec(&resp, standard()) {
                    Ok(data) => {
                        if data.len() > MAX_PAYLOAD_SIZE {
                            let err_resp =
                                Response::Error("Response too large for UDP".to_string());
                            if let Ok(err_data) = encode_to_vec(&err_resp, standard()) {
                                let _ = socket.send_to(&err_data, src_addr);
                            }
                        } else {
                            match socket.send_to(&data, src_addr) {
                                Ok(sent) => println!("Sent {} bytes to {}", sent, src_addr),
                                Err(e) => eprintln!("Send error: {}", e),
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Encode error: {}", e);
                        let err_resp = Response::Error(format!("Encode error: {}", e));
                        if let Ok(err_data) = encode_to_vec(&err_resp, standard()) {
                            let _ = socket.send_to(&err_data, src_addr);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Receive error: {}", e);
            }
        }
    }
}
