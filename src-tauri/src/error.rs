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
    TokioJoinError(#[from] tokio::task::JoinError),

    #[error(transparent)]
    Pyo3Error(#[from] pyo3::PyErr),

    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),

    #[error("The {0} script was not found.")]
    ScriptNotFound(String),
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
