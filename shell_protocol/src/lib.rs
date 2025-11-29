use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Ok,
    DirList(Vec<DirEntry>),
    CopyResult { bytes_copied: u64 },
    FileMetadata { name: String, size: u64 },
    Error(String),
}
