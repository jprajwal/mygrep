use std::convert::From;
use std::error;
use std::fmt;

#[derive(Debug)]
pub struct GrepError {
    msg: String,
}

impl GrepError {
    pub fn from_err<T: error::Error>(e: T) -> Self {
        GrepError {
            msg: format!("{}", e),
        }
    }
}

impl From<&str> for GrepError {
    fn from(value: &str) -> Self {
        return GrepError {
            msg: value.to_string(),
        };
    }
}

impl From<String> for GrepError {
    fn from(value: String) -> Self {
        return GrepError { msg: value };
    }
}

impl std::fmt::Display for GrepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mygrep: {}", self.msg)
    }
}

impl error::Error for GrepError {}
