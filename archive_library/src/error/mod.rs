use crate::error::tar_error::TarError;
use std::{fmt, result};

pub mod tar_error;

pub struct Error {
    error_type: ErrorType,
}
impl Error {
    pub fn new(kind: ErrorType) -> Error {
        Error { error_type: kind }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Archive Util Error - {}", self.error_type)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error_type)
    }
}

impl From<tar_error::TarError> for Error {
    fn from(error: tar_error::TarError) -> Self {
        Self::new(ErrorType::Tar(error))
    }
}

pub type Result<T> = result::Result<T, Error>;

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.error_type, f)
    }
}

#[derive(Debug)]
pub enum ErrorType {
    Tar(TarError),
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorType::Tar(t) => write!(f, "TAR: {:?}", t),
        }
    }
}

impl std::error::Error for ErrorType {}
