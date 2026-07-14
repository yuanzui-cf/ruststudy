/// It's an input macro
#[macro_export]
macro_rules! __utils_io_input {
    () => {
        (|| -> $crate::io::error::Result<String> {
            use std::io;
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)?;
            Ok(input.trim().to_string())
        })()
    };

    ($first:ty, $($rest:ty),+) => {
        (|| -> $crate::io::error::Result<($first, $($rest),+)> {
            use std::io;
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)?;
            let mut parts = input.split_whitespace();

            Ok((
                parts.next()
                    .ok_or($crate::io::error::Error::MissingValue)?
                    .parse::<$first>()
                    .map_err(|e| $crate::io::error::Error::Parse(e.to_string()))?,
                $(
                    parts.next()
                        .ok_or($crate::io::error::Error::MissingValue)?
                        .parse::<$rest>()
                        .map_err(|e| $crate::io::error::Error::Parse(e.to_string()))?,
                ),
                +
            ))
        })()
    };

    ($type:ty) => {
        (|| -> $crate::io::error::Result<$type> {
            use std::io;
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)?;
            input.trim()
                .parse::<$type>()
                .map_err(|e| $crate::io::error::Error::Parse(e.to_string()))
        })()
    };

    ($prompt:expr, $type:ty) => {
        (|| -> $crate::io::error::Result<$type> {
            use std::io::{self, Write};
            print!("{}", $prompt);
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)?;
            input.trim()
                .parse::<$type>()
                .map_err(|e| $crate::io::error::Error::Parse(e.to_string()))
        })()
    };
}

pub use crate::__utils_io_input as input;
