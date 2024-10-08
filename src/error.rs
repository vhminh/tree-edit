use std::{error::Error, fmt::Display, io};

#[derive(Debug)]
pub enum TreeEditError {
    DuplicatePath(String),
    InvalidEntry(String),
    InvalidFileId(u64),
    FsChanged(DetectedBy),
    IOError(io::Error),
}

impl Error for TreeEditError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            TreeEditError::DuplicatePath(_) => None,
            TreeEditError::InvalidEntry(_) => None,
            TreeEditError::InvalidFileId(_) => None,
            TreeEditError::FsChanged(_) => None,
            TreeEditError::IOError(ref source) => Some(source),
        }
    }
}

impl Display for TreeEditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TreeEditError::DuplicatePath(path) => write!(f, "duplicate path {}", path),
            TreeEditError::InvalidEntry(entry) => write!(f, "invalid entry {}", entry),
            TreeEditError::InvalidFileId(id) => write!(f, "invalid file id {}", id),
            TreeEditError::FsChanged(detected_by) => {
                write!(f, "file system changed while editing: {}", detected_by)
            }
            TreeEditError::IOError(ref source) => source.fmt(f),
        }
    }
}

impl From<io::Error> for TreeEditError {
    fn from(source: io::Error) -> Self {
        TreeEditError::IOError(source)
    }
}

#[derive(Debug)]
pub enum DetectedBy {
    FileNotFound(String),
    FileExists(String),
}

impl Display for DetectedBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectedBy::FileNotFound(path) => write!(f, "file not found {}", path),
            DetectedBy::FileExists(path) => write!(f, "file already exists {}", path),
        }
    }
}
