use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
    time::Instant,
};

use chatgpt::{client::ChatGPT, converse::Conversation};
use ollama_rs::{generation::completion::GenerationContext, Ollama};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use super::GameParser;

#[derive(Default)]
pub struct AppState {
    pub running_thread: Option<std::thread::JoinHandle<()>>,
    pub config: Config,
    pub stop_flag: Arc<AtomicBool>,

    /// This is the id of the command
    pub disabled_commands: Option<Arc<Mutex<Vec<String>>>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub file_path: String,
    pub command_timeout: u64,
    pub owner: String,
    pub parser: GameParser,
    pub openai_api_key: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Command {
    pub enabled: bool,
    pub id: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    pub enabled: bool,
    pub id: String,
    pub name: String,
    pub description: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            file_path: String::from(""),
            command_timeout: 10,
            owner: String::from(""),
            parser: GameParser::CounterStrike2,
            openai_api_key: String::from(""),
        }
    }
}

pub struct UserCooldown {
    pub timestamps: Vec<Instant>,
}

pub struct CmdState {
    // Chat GPT Related
    pub chat_gpt: ChatGPT,
    pub conversations: HashMap<String, Conversation>,
    pub personality: String,

    // Ollama related
    pub ollama: Ollama,
    pub message_context: HashMap<String, GenerationContext>,

    // Eval related
    pub user_cooldowns: HashMap<String, UserCooldown>,

    pub disabled_commands: Arc<Mutex<Vec<String>>>,
}