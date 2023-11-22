// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod lexer;
mod model;

use std::{
    collections::HashMap,
    env,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use chatgpt::{
    prelude::{ChatGPT, Conversation},
    types::CompletionResponse,
};
use log::{info, warn, LevelFilter};
use model::state::{AppState, CmdState, Config};
use ollama_rs::{
    generation::completion::{request::GenerationRequest, GenerationContext},
    Ollama,
};
use serde::{Deserialize, Serialize};
use source_cmd_parser::{
    log_parser::{SouceError, SourceCmdLogParser},
    model::{ChatMessage, ChatResponse},
};
use tauri::State;
use tokio::sync::{Mutex, RwLock};

use crate::model::state::UserCooldown;

const CONFIG_FILE: &str = "config.json";

#[tauri::command]
async fn is_running(state: State<'_, Arc<Mutex<AppState>>>) -> Result<bool, ()> {
    Ok(state.lock().await.running_thread.is_some())
}

#[tauri::command]
async fn get_config(state: State<'_, Arc<Mutex<AppState>>>) -> Result<Config, ()> {
    // Load config from file
    if let Ok(config_json) = tokio::fs::read_to_string(CONFIG_FILE).await {
        let config: Config = serde_json::from_str(&config_json).unwrap();
        state.lock().await.config = config.clone();
    }

    Ok(state.lock().await.config.clone())
}

async fn save_config(state: State<'_, Arc<Mutex<AppState>>>, config: Config) {
    state.lock().await.config = config;

    // Save config to file as json
    let config_json = serde_json::to_string(&state.lock().await.config).unwrap();

    tokio::fs::write(CONFIG_FILE, config_json)
        .await
        .expect("Failed to save config to file");
}

#[tauri::command]
async fn start(state: State<'_, Arc<Mutex<AppState>>>, config: Config) -> Result<(), ()> {
    // Save config to file as json
    save_config(state.clone(), config.clone()).await;

    let mut state = state.lock().await;
    if state.running_thread.is_some() {
        return Err(());
    }

    state.stop_flag = Arc::new(AtomicBool::new(false));

    let stop_flag = state.stop_flag.clone();
    let api_key = config.openai_api_key.clone();
    let handle = std::thread::spawn(move || {
        let chat_gpt = ChatGPT::new(api_key).expect("Unable to create GPT Client");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut parser = SourceCmdLogParser::builder()
                .file_path(Box::new(PathBuf::from(config.file_path)))
                .state(CmdState {
                    personality: String::new(),
                    chat_gpt,
                    conversations: HashMap::new(),
                    ollama: Ollama::default(),
                    message_context: HashMap::new(),
                    user_cooldowns: HashMap::new(),
                })
                .set_parser(config.parser.get_parser())
                .add_command(".ping", pong)
                .add_command(".explain", explain)
                .add_command(".personality", personality)
                .add_command(".llama2", llama2)
                .add_global_command(eval)
                .stop_flag(stop_flag)
                .time_out(Duration::from_secs(config.command_timeout))
                .build()
                .expect("Failed to build parser");

            parser.run().await.unwrap();
        });
    });

    state.running_thread = Some(handle);

    Ok(())
}

#[tauri::command]
async fn stop(state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), ()> {
    let mut state = state.lock().await;
    if let Some(handle) = state.running_thread.take() {
        state.stop_flag.store(true, Ordering::Relaxed);
        handle.join().unwrap();
    }

    Ok(())
}

async fn pong(
    chat_message: ChatMessage,
    _: Arc<RwLock<CmdState>>,
) -> Result<Option<ChatResponse>, SouceError> {
    Ok(Some(ChatResponse::new(format!(
        "PONG {}",
        chat_message.message
    ))))
}

fn main() {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(LevelFilter::Debug)
        .init();

    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(AppState::default())))
        .invoke_handler(tauri::generate_handler![
            is_running, get_config, start, stop
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn explain(
    chat_message: ChatMessage,
    state: Arc<RwLock<CmdState>>,
) -> Result<Option<ChatResponse>, SouceError> {
    info!("Explain: {}", chat_message.message);

    let mut personality = state.read().await.personality.clone();

    let response: CompletionResponse = state.read().await.chat_gpt
        .send_message(format!(
            "Please response in 120 characters or less. Can you response as if you were {}. The prompt is: \"{}\"",
            personality,
            chat_message.message
        ))
        .await?;

    if !personality.is_empty() {
        personality = format!(" {}", personality);
    }

    let mut chat_response = format!("[AI{}]: ", personality);

    chat_response.push_str(response.message_choices[0].message.content.as_str());

    Ok(Some(ChatResponse::new(chat_response)))
}
async fn personality(
    chat_message: ChatMessage,
    state: Arc<RwLock<CmdState>>,
) -> Result<Option<ChatResponse>, SouceError> {
    let message = chat_message.message;

    let mut state = state.write().await;

    state.personality = message;

    Ok(None)
}

async fn llama2(
    message: ChatMessage,
    state: Arc<RwLock<CmdState>>,
) -> Result<Option<ChatResponse>, SouceError> {
    let mut state = state.write().await;

    let mut request = GenerationRequest::new(
        "llama2-uncensored:latest".to_string(),
        format!(
            "Please keep the response under 120 characters. {} Says \"{}\"",
            message.user_name, message.message
        ),
    );

    if let Some(context) = state.message_context.get(&message.user_name) {
        request = request.context(context.clone());
    }

    let response = state.ollama.generate(request).await;

    if let Ok(response) = response {
        state.message_context.insert(
            message.user_name.clone(),
            response.final_data.unwrap().context,
        );
        Ok(Some(ChatResponse::new(response.response)))
    } else {
        Ok(None)
    }
}

const COOLDOWN_DURATION: Duration = Duration::from_secs(120); // 2 minutes
const MESSAGE_LIMIT: usize = 50;

async fn eval(
    chat_message: ChatMessage,
    state_lock: Arc<RwLock<CmdState>>,
) -> Result<Option<ChatResponse>, SouceError> {
    let message = chat_message.raw_message;

    if message.trim().parse::<f64>().is_ok() {
        return Ok(None);
    }

    info!("{} said {} ", chat_message.user_name, message);

    let tokens = lexer::tokenize(message.as_str());

    let expression = lexer::to_string(&tokens);

    if expression.is_empty()
        || tokens.len() == 1
        || tokens
            .iter()
            .all(|token| token.is_number() || token.is_parathesis())
    {
        return Ok(None);
    }

    info!("Eval: {}", expression);

    match meval::eval_str(&expression.replace('x', "*")) {
        Ok(response) => {
            {
                let mut state = state_lock.write().await;

                let user_cooldown = state
                    .user_cooldowns
                    .entry(chat_message.user_name.clone())
                    .or_insert(UserCooldown {
                        timestamps: Vec::new(),
                    });

                // Remove outdated timestamps
                user_cooldown
                    .timestamps
                    .retain(|&timestamp| timestamp.elapsed() < COOLDOWN_DURATION);

                // Check cooldown status
                if user_cooldown.timestamps.len() >= MESSAGE_LIMIT {
                    warn!(
                        "Skipping eval. User {} has reached the message limit of {}. Time left till cooldown: {:?}",
                        chat_message.user_name, MESSAGE_LIMIT,
                        COOLDOWN_DURATION - user_cooldown.timestamps[0].elapsed());

                    return Ok(None);
                }

                // If not in cooldown, add the new timestamp
                user_cooldown.timestamps.push(Instant::now());
            }

            if message.contains("[Store]") {
                return Ok(Some(ChatResponse::new(format!(
                    "The answer is:  {}",
                    response
                ))));
            }

            info!("Eval: {} = {}", message, response);
            Ok(Some(ChatResponse::new(response.to_string())))
        }
        Err(_) => Ok(None),
    }
}
