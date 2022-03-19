//! Define custom error type.

// use thiserror::Error;

/// Error enumerates possible errors returned by this library.
#[derive(thiserror::Error, Debug)]
pub enum CError {
    #[error("No comment sign found for file {0}")]
    UnknownCommentSign(String),

    #[error("Error while running git subcommand: {0}")]
    GitCmdError(String),

    #[error("Invalid configuration")]
    ConfigError(String),

    #[error("Could not read {0}")]
    ReadError(String),

    #[error("Could not write {0}")]
    WriteError(String),

    #[error("Some copyrights could not be fixed, please check the output")]
    FixError,

    #[error("The copyright job changed tracked files that should be committed")]
    FilesChanged,

    #[error(transparent)]
    GenericIOError(#[from] std::io::Error),

    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error(transparent)]
    RegexError(#[from] regex::Error),
}
