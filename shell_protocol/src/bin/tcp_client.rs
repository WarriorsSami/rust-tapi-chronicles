use bincode::config::standard;
use bincode::serde::{decode_from_std_read, encode_into_std_write};
use shell_protocol::{DirEntry, Request, Response};
use std::fs::File;
use std::io::{self, BufRead, Read, Write};
use std::net::TcpStream;

fn send_request(stream: &mut TcpStream, req: &Request) -> io::Result<Response> {
    encode_into_std_write(req, stream, standard())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("encode error: {e}")))?;
    let resp: Response = decode_from_std_read(stream, standard())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("decode error: {e}")))?;
    Ok(resp)
}

fn do_upload(stream: &mut TcpStream, local_path: &str, remote_folder: &str) -> io::Result<()> {
    let mut f = File::open(local_path)?;
    let metadata = f.metadata()?;
    let size = metadata.len();
    let filename = std::path::Path::new(local_path)
        .file_name()
        .and_then(|os| os.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid filename"))?
        .to_string();

    let req = Request::Upload {
        dst_path: remote_folder.to_string(),
        file_name: filename.clone(),
        size,
    };

    // Send the upload request
    encode_into_std_write(&req, stream, standard())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("encode error: {e}")))?;

    // Wait for server acknowledgment
    let resp: Response = decode_from_std_read(stream, standard())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("decode error: {e}")))?;

    match resp {
        Response::Ok => {
            // Server is ready, now send the file data
            let bytes_written = io::copy(&mut f, stream)?;
            stream.flush()?;
            println!("Uploaded {} ({} bytes)", filename, bytes_written);
        }
        Response::Error(msg) => {
            eprintln!("Upload error: {}", msg);
            return Err(io::Error::new(io::ErrorKind::Other, msg));
        }
        _ => {
            eprintln!("Unexpected response to upload: {:?}", resp);
            return Err(io::Error::new(io::ErrorKind::Other, "Unexpected response"));
        }
    }
    Ok(())
}

fn do_download(stream: &mut TcpStream, remote_path: &str, local_folder: &str) -> io::Result<()> {
    let req = Request::Download {
        src_path: remote_path.to_string(),
    };

    // Send download request
    encode_into_std_write(&req, stream, standard())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("encode error: {e}")))?;

    // Wait for server response with metadata
    let resp: Response = decode_from_std_read(stream, standard())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("decode error: {e}")))?;

    match resp {
        Response::FileMetadata { name, size } => {
            let local_path = std::path::Path::new(local_folder).join(&name);
            let mut f = File::create(&local_path)?;
            let mut remaining = size;
            let mut buf = [0u8; 8192];
            let mut total_read = 0u64;

            while remaining > 0 {
                let to_read = std::cmp::min(buf.len() as u64, remaining) as usize;
                let n = stream.read(&mut buf[..to_read])?;
                if n == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        format!("Connection closed after {} of {} bytes", total_read, size)
                    ));
                }
                f.write_all(&buf[..n])?;
                remaining -= n as u64;
                total_read += n as u64;
            }
            f.flush()?;
            println!(
                "Downloaded {} ({} bytes) → {}",
                name,
                size,
                local_path.display()
            );
        }
        Response::Error(msg) => {
            eprintln!("Download error: {}", msg);
            return Err(io::Error::new(io::ErrorKind::Other, msg));
        }
        _ => {
            eprintln!("Unexpected response: {:?}", resp);
            return Err(io::Error::new(io::ErrorKind::Other, "Unexpected response"));
        }
    }
    Ok(())
}

fn print_dir_list(list: Vec<DirEntry>) {
    for e in list {
        println!("{}{}", e.name, if e.is_dir { "/" } else { "" });
    }
}

fn main() -> io::Result<()> {
    let mut input = String::new();
    print!("Server address (host:port): ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;
    let addr = input.trim();

    let mut stream = TcpStream::connect(addr)?;
    println!("Connected to {}", addr);

    let stdin = io::stdin();
    print!("> ");
    io::stdout().flush()?;
    for line in stdin.lock().lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            print!("> ");
            io::stdout().flush()?;
            continue;
        }

        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("dir") => {
                if let Ok(resp) = send_request(&mut stream, &Request::Dir) {
                    if let Response::DirList(list) = resp {
                        print_dir_list(list);
                    } else {
                        println!("Response: {:?}", resp);
                    }
                }
            }

            Some("cd") => {
                if let Some(arg) = parts.next() {
                    let req = if arg == ".." {
                        Request::CdUp
                    } else {
                        Request::Cd {
                            path: arg.to_string(),
                        }
                    };
                    if let Ok(resp) = send_request(&mut stream, &req) {
                        println!("{:?}", resp);
                    }
                } else {
                    println!("Usage: cd <path> or cd ..");
                }
            }

            Some("mkdir") => {
                if let Some(name) = parts.next() {
                    if let Ok(resp) = send_request(
                        &mut stream,
                        &Request::Mkdir {
                            name: name.to_string(),
                        },
                    ) {
                        println!("{:?}", resp);
                    }
                } else {
                    println!("Usage: mkdir <folder>");
                }
            }

            Some("copy") => {
                if let (Some(src), Some(dst)) = (parts.next(), parts.next()) {
                    if let Ok(resp) = send_request(
                        &mut stream,
                        &Request::Copy {
                            src: src.to_string(),
                            dst: dst.to_string(),
                        },
                    ) {
                        println!("{:?}", resp);
                    }
                } else {
                    println!("Usage: copy <src> <dst>");
                }
            }

            Some("upload") => {
                if let (Some(local), Some(remote_folder)) = (parts.next(), parts.next()) {
                    let _ = do_upload(&mut stream, local, remote_folder);
                } else {
                    println!("Usage: upload <local_path> <remote_folder_on_server>");
                }
            }

            Some("download") => {
                if let (Some(remote_path), Some(local_folder)) = (parts.next(), parts.next()) {
                    let _ = do_download(&mut stream, remote_path, local_folder);
                } else {
                    println!("Usage: download <remote_path_on_server> <local_folder>");
                }
            }

            Some("exit") | Some("quit") => {
                println!("Exiting.");
                break;
            }

            Some(cmd) => {
                println!("Unknown command: {}", cmd);
            }

            None => {}
        }

        print!("> ");
        io::stdout().flush()?;
    }

    Ok(())
}
