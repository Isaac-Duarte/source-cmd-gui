// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod lexer;
mod logger;
mod model;

use std::{
    collections::HashMap,
    env,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use chatgpt::prelude::ChatGPT;
use lazy_static::lazy_static;
use log::{info, LevelFilter};
use logger::Log;
use model::state::{AppState, CmdState, CommandResponse, Config};
use ollama_rs::Ollama;

use source_cmd_parser::log_parser::SourceCmdLogParser;
use tauri::{State, Manager};
use tokio::sync::{mpsc, Mutex};

lazy_static! {
    static ref CONFIG_FILE: PathBuf = {
        let home_dir = env::var("HOME").expect("HOME not found");
        PathBuf::from(home_dir).join(".souce-cmd-gui-config.json")
    };
}

#[tauri::command]
async fn is_running(state: State<'_, Arc<Mutex<AppState>>>) -> Result<bool, ()> {
    Ok(state.lock().await.running_thread.is_some())
}

#[tauri::command]
async fn get_config(state: State<'_, Arc<Mutex<AppState>>>) -> Result<Config, ()> {
    // Load config from file
    if let Ok(config_json) = tokio::fs::read_to_string(CONFIG_FILE.clone()).await {
        let config: Config = serde_json::from_str(&config_json).unwrap();
        state.lock().await.config = config.clone();
    }

    Ok(state.lock().await.config.clone())
}

#[tauri::command]
async fn save_config(state: State<'_, Arc<Mutex<AppState>>>, config: Config) -> Result<(), ()> {
    state.lock().await.config = config;

    // Save config to file as json
    let config_json = serde_json::to_string(&state.lock().await.config).unwrap();

    tokio::fs::write(CONFIG_FILE.clone(), config_json)
        .await
        .expect("Failed to save config to file");

    info!("Saved config to file");

    Ok(())
}

/// TODO: Pull this from the parser directly
#[tauri::command]
fn get_commands() -> Vec<CommandResponse> {
    vec![
        CommandResponse {
            enabled: true,
            id: String::from("ping"),
            name: String::from("Ping"),
            description: String::from("Pong!"),
        },
        CommandResponse {
            enabled: true,
            id: String::from("explain"),
            name: String::from("Explain"),
            description: String::from("Explain something to the AI"),
        },
        CommandResponse {
            enabled: true,
            id: String::from("personality"),
            name: String::from("Personality"),
            description: String::from("Set the AI's personality"),
        },
        CommandResponse {
            enabled: true,
            id: String::from("llama2"),
            name: String::from("Llama2"),
            description: String::from("Generate a response from Llama2 (Requires OLlama)"),
        },
        CommandResponse {
            enabled: true,
            id: String::from("eval"),
            name: String::from("Eval"),
            description: String::from("Evaluate a math expression"),
        },
    ]
}

#[tauri::command]
async fn start(state: State<'_, Arc<Mutex<AppState>>>, config: Config) -> Result<(), ()> {
    let mut state = state.lock().await;
    if state.running_thread.is_some() {
        return Err(());
    }

    state.stop_flag = Arc::new(AtomicBool::new(false));

    let stop_flag = state.stop_flag.clone();
    let api_key = config.openai_api_key.clone();

    let chat_gpt = ChatGPT::new(api_key).expect("Unable to create GPT Client");

    let cmd_state = CmdState {
        personality: String::new(),
        chat_gpt,
        conversations: HashMap::new(),
        ollama: Ollama::default(),
        message_context: HashMap::new(),
        user_cooldowns: HashMap::new(),
        disabled_commands: state.disabled_commands.clone(),
    };

    let handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut parser = SourceCmdLogParser::builder()
                .file_path(Box::new(PathBuf::from(config.file_path)))
                .state(cmd_state)
                .set_parser(config.parser.get_parser())
                .add_command(".ping", commands::pong)
                .add_command(".explain", commands::explain)
                .add_command(".personality", commands::personality)
                .add_command(".llama2", commands::llama2)
                .add_global_command(commands::eval)
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

#[tauri::command]
async fn update_disabled_commands(
    state: State<'_, Arc<Mutex<AppState>>>,
    disabled_commands: Vec<String>,
) -> Result<(), ()> {
    info!("Updating disabled commands, {:?}", disabled_commands);
    let commands = state.lock().await.disabled_commands.clone();

    // We need to mutablly change the commands
    let mut commands = commands.lock().await;
    commands.clear();
    commands.extend(disabled_commands);

    Ok(())
}

fn main() {
    let (tx, mut rx) = mpsc::channel::<Log>(100);

    logger::setup_logger(tx);

    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(AppState::default())))
        .invoke_handler(tauri::generate_handler![
            is_running,
            get_config,
            start,
            stop,
            get_commands,
            update_disabled_commands,
            save_config
        ])
        .setup(move |app| {
            let app_handle = app.handle();
            tauri::async_runtime::spawn(async move {
                while let Some(message) = rx.recv().await {
                    app_handle.emit_all("stdout_data", &message).unwrap();
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
