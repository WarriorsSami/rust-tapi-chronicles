use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
pub enum Request {
    Dir,
    CdUp,
    Mkdir {
        name: String,
    },
    Cd {
        path: String,
    },
    Copy {
        src: String,
        dst: String,
    },
    /// Upload (client → server): after sending metadata, client will stream raw bytes.
    Upload {
        dst_path: String,
        file_name: String,
        size: u64,
    },
    /// Download (server → client): server responds with metadata, then streams raw file bytes.
    Download {
        src_path: String,
    },
    /// UDP Upload: client sends file chunk
    UploadChunk {
        chunk_id: u32,
        data: Vec<u8>,
        is_last: bool,
    },
    /// UDP Download: request next chunk
    DownloadChunk {
        chunk_id: u32,
    },
}

#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
pub enum Response {
    Ok,
    DirList(Vec<DirEntry>),
    CopyResult {
        bytes_copied: u64,
    },
    FileMetadata {
        name: String,
        size: u64,
    },
    Error(String),
    /// UDP: Acknowledge chunk received
    ChunkAck {
        chunk_id: u32,
    },
    /// UDP: Send file chunk
    FileChunk {
        chunk_id: u32,
        data: Vec<u8>,
        is_last: bool,
    },
}
