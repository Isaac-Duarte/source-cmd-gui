use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};

use chatgpt::{client::ChatGPT, converse::Conversation};

use serde::{Deserialize, Serialize};

use crate::{error::SourceCmdGuiResult, python::DynamicPythonCtx, repository::JsonRepository};

use super::GameParser;

pub struct AppState {
    pub running_thread: Option<std::thread::JoinHandle<SourceCmdGuiResult>>,
    pub config: Config,
    pub stop_flag: Arc<AtomicBool>,
    pub cmd_state: CmdState,
    pub script_repository: JsonRepository,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub file_path: String,
    pub command_timeout: u64,
    pub owner: String,
    pub parser: GameParser,
    pub openai_api_key: String,
    pub disabled_commands: Vec<String>,
    pub response_direction: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            file_path: String::from(""),
            command_timeout: 10,
            owner: String::from(""),
            parser: GameParser::CounterStrike2,
            openai_api_key: String::from(""),
            disabled_commands: vec![],
            response_direction: "Keep the response to 120 chars".to_string(),
        }
    }
}

#[derive(Default)]
pub struct CmdState {
    // Chat GPT Related
    pub chat_gpt: Option<ChatGPT>,
    pub conversations: HashMap<String, Conversation>,
    pub personality: String,

    // Dynamic context for python
    pub python_context: DynamicPythonCtx,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    pub enabled: bool,
    pub id: String,
    pub name: String,
    pub description: String,
}
