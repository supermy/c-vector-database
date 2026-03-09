use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    DimensionMismatch { expected: usize, got: usize },
    DuplicateId(i64),
    NotFound(i64),
    CapacityExceeded,
    InvalidVector,
    IoError(io::Error),
    SerializationError(String),
    InvalidDistanceMetric,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::DimensionMismatch { expected, got } => {
                write!(f, "Dimension mismatch: expected {}, got {}", expected, got)
            }
            Error::DuplicateId(id) => write!(f, "Duplicate vector ID: {}", id),
            Error::NotFound(id) => write!(f, "Vector not found: {}", id),
            Error::CapacityExceeded => write!(f, "Database capacity exceeded"),
            Error::InvalidVector => write!(f, "Invalid vector"),
            Error::IoError(e) => write!(f, "IO error: {}", e),
            Error::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Error::InvalidDistanceMetric => write!(f, "Invalid distance metric"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IoError(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
