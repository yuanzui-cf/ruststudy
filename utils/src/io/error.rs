use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Parse Error: {0}")]
    Parse(String),

    #[error("Insufficient input values")]
    MissingValue,
}

/// Result of IO
pub type Result<T> = std::result::Result<T, Error>;
