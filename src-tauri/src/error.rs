use serde::Serialize;

pub type SourceCmdGuiResult<T = ()> = std::result::Result<T, SourceCmdGuiError>;

#[derive(thiserror::Error, Debug)]
pub enum SourceCmdGuiError {
    #[error(transparent)]
    SourceCmdParserError(#[from] source_cmd_parser::error::SourceCmdError),

    #[error("SourceCmdParser is already running")]
    ProcessAlreadyRunning,

    #[error(transparent)]
    TauriError(#[from] tauri::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    ChatGptError(#[from] chatgpt::err::Error),

    #[error(transparent)]
    RusqliteError(#[from] rusqlite::Error),

    #[error(transparent)]
    TokioRusqlite(#[from] tokio_rusqlite::Error),

    #[error(transparent)]
    TokioJoinError(#[from] tokio::task::JoinError),

    #[error(transparent)]
    Pyo3Error(#[from] pyo3::PyErr),
}

impl SourceCmdGuiError {
    pub fn to_string(&self) -> String {
        format!("{}", self)
    }
}

impl Serialize for SourceCmdGuiError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
