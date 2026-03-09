use std::fmt;

#[derive(Debug)]
pub enum Error {
    InvalidDimension { expected: usize, got: usize },
    DuplicateId(u64),
    NotFound(u64),
    InvalidParameter(String),
    IoError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimension { expected, got } => {
                write!(f, "Invalid dimension: expected {}, got {}", expected, got)
            }
            Self::DuplicateId(id) => write!(f, "Duplicate ID: {}", id),
            Self::NotFound(id) => write!(f, "ID not found: {}", id),
            Self::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
