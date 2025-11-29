use bincode::config::standard;
use bincode::serde::{decode_from_std_read, encode_into_std_write};
use shell_protocol::{DirEntry, Request, Response};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

fn send_response(stream: &mut TcpStream, resp: &Response) -> std::io::Result<()> {
    encode_into_std_write(resp, stream, standard()).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("encode error: {e}"))
    })?;
    Ok(())
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<Request> {
    decode_from_std_read(stream, standard()).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("decode error: {e}"),
        )
    })
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
        _ => Response::Error("Unexpected request in FS handler".into()),
    }
}

fn handle_client(mut stream: TcpStream, root: PathBuf) -> std::io::Result<()> {
    let mut cwd = root.clone();

    loop {
        let req = match read_request(&mut stream) {
            Ok(r) => r,
            Err(_) => break, // assume connection closed or bad data → exit
        };

        match req {
            Request::Upload {
                dst_path,
                file_name,
                size,
            } => {
                // Build destination path properly
                let dest = if dst_path == "." || dst_path.is_empty() {
                    cwd.join(&file_name)
                } else {
                    cwd.join(&dst_path).join(&file_name)
                };
                
                if let Some(parent) = dest.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                
                match File::create(&dest) {
                    Ok(mut f) => {
                        // Send OK response to acknowledge we're ready to receive
                        send_response(&mut stream, &Response::Ok)?;
                        
                        let mut remaining = size;
                        let mut buf = [0u8; 8192];
                        while remaining > 0 {
                            let to_read = std::cmp::min(buf.len() as u64, remaining) as usize;
                            let n = stream.read(&mut buf[..to_read])?;
                            if n == 0 {
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::UnexpectedEof,
                                    "EOF during file upload",
                                ));
                            }
                            f.write_all(&buf[..n])?;
                            remaining -= n as u64;
                        }
                        println!("Uploaded file {} to {}", file_name, dest.display());
                    }
                    Err(e) => {
                        send_response(
                            &mut stream,
                            &Response::Error(format!("Cannot create file: {}", e)),
                        )?;
                    }
                }
            }

            Request::Download { src_path } => {
                let full = cwd.join(src_path);
                match File::open(&full) {
                    Ok(mut f) => {
                        let metadata = f.metadata()?;
                        let size = metadata.len();
                        let name = full
                            .file_name()
                            .and_then(|os| os.to_str())
                            .unwrap_or("file")
                            .to_string();

                        send_response(
                            &mut stream,
                            &Response::FileMetadata {
                                name: name.clone(),
                                size,
                            },
                        )?;
                        let bytes_sent = std::io::copy(&mut f, &mut stream)?;
                        stream.flush()?;
                        println!("Sent file {} ({} bytes)", name, bytes_sent);
                    }
                    Err(e) => {
                        send_response(
                            &mut stream,
                            &Response::Error(format!("Open failed: {}", e)),
                        )?;
                    }
                }
            }

            other => {
                let resp = handle_fs_request(&mut cwd, &root, other);
                send_response(&mut stream, &resp)?;
            }
        }
    }

    Ok(())
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: server <addr:port> <root_dir>");
        std::process::exit(1);
    }
    let addr = &args[1];
    let root = PathBuf::from(&args[2]);

    let listener = TcpListener::bind(addr)?;
    println!("Server listening on {}", addr);

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                println!("Client connected: {}", s.peer_addr()?);
                if let Err(e) = handle_client(s, root.clone()) {
                    eprintln!("Client handler error: {:?}", e);
                }
                println!("Client disconnected");
                break; // single-client only
            }
            Err(e) => {
                eprintln!("Accept error: {:?}", e);
            }
        }
    }

    Ok(())
}
