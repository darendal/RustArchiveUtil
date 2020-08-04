use std::{error, result};

pub type Result<T> = result::Result<T, TarError>;

#[derive(Debug)]
pub enum TarErrorKind {
    EmptyHeaderBlock,
    InvalidChecksum,
    InvalidMagicValue,
    IOError(std::io::Error),
    InvalidFormatDirectory,
    InvalidFormatWrongExtension,
    InvalidFormatMissingExtension,
}

#[derive(Debug)]
pub struct TarError {
    pub kind: TarErrorKind,
    pub error: Box<dyn error::Error + Send + Sync>,
}

impl TarError {
    pub fn new<E>(kind: TarErrorKind, error: E) -> TarError
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Self::_new(kind, error.into())
    }

    fn _new(kind: TarErrorKind, error: Box<dyn error::Error + Send + Sync>) -> TarError {
        TarError { kind, error }
    }
}

impl From<std::io::Error> for TarError {
    fn from(error: std::io::Error) -> Self {
        TarError::new(TarErrorKind::IOError(error), "")
    }
}
