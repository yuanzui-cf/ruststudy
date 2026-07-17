use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("SyntaxError: {0}")]
    Syntax(String),
    #[error("TypeError: {0}")]
    Type(String),
    #[error("NameError: {0}")]
    Name(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[doc(hidden)]
#[macro_export]
macro_rules! __error {
    ($ty:ident, $fmt:expr $(, $arg:expr)* $(,)?) => {
        $crate::error::Error::$ty(format!($fmt $(, $arg)*))
    };
}

pub use crate::__error as error;
