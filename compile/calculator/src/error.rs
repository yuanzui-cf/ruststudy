use thiserror::Error;

use crate::ast::Value;

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("SyntaxError: {0}")]
    Syntax(String),
    #[error("TypeError: {0}")]
    Type(String),
    #[error("NameError: {0}")]
    Name(String),
    #[error("RuntimeError: {0}")]
    Runtime(String),

    #[error("InternalError")]
    Internal(InternalError),
}

#[derive(Debug, Clone)]
pub enum InternalError {
    LoopBreak(Option<Value>),
    LoopContinue,
    FunctionReturn(Option<Value>),
}

pub type Result<T> = std::result::Result<T, Error>;

#[doc(hidden)]
#[macro_export]
macro_rules! __error {

    (Internal, $err:expr $(,)?) => {
        $crate::error::Error::Internal($err)
    };

    ($ty:ident, $fmt:expr $(, $arg:expr)* $(,)?) => {
        $crate::error::Error::$ty(format!($fmt $(, $arg)*))
    };
}

pub use crate::__error as error;
