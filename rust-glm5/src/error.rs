use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub enum Error {
    InvalidDimension { expected: usize, got: usize },
    DuplicateId(u64),
    NotFound(u64),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimension { expected, got } => {
                write!(f, "Invalid dimension: expected {}, got {}", expected, got)
            }
            Self::DuplicateId(id) => write!(f, "Duplicate id: {}", id),
            Self::NotFound(id) => write!(f, "Id {} not found", id),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
