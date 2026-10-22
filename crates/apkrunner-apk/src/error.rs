use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Zip(zip::result::ZipError),
    /// The APK manifest is missing, malformed or unsupported.
    Manifest(String),
    /// The file is not a valid APK at all.
    InvalidApk(String),
    /// A path inside the ZIP tries to escape the destination directory.
    UnsafeEntry(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::Zip(e) => write!(f, "invalid ZIP: {e}"),
            Error::Manifest(m) => write!(f, "invalid AndroidManifest.xml: {m}"),
            Error::InvalidApk(m) => write!(f, "not a valid APK: {m}"),
            Error::UnsafeEntry(name) => {
                write!(f, "refusing unsafe archive entry: {name}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Error::Zip(e)
    }
}
