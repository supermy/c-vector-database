use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    NotFound,
    InvalidDimension,
    DuplicateId,
    OutOfMemory,
    InvalidInput,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "Entry not found"),
            Self::InvalidDimension => write!(f, "Invalid vector dimension"),
            Self::DuplicateId => write!(f, "Duplicate ID"),
            Self::OutOfMemory => write!(f, "Out of memory"),
            Self::InvalidInput => write!(f, "Invalid input"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Error::InvalidInput
    }
}

impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Self {
        Error::InvalidInput
    }
}

pub type Result<T> = std::result::Result<T, Error>;
