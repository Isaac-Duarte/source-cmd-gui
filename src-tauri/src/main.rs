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
use log::info;
use logger::Log;
use model::state::{AppState, CmdState, CommandResponse, Config};
use ollama_rs::Ollama;

use source_cmd_parser::log_parser::SourceCmdLogParser;
use tauri::{Manager, State};
use tokio::sync::{mpsc, Mutex};

lazy_static! {
    static ref CONFIG_FILE: PathBuf = {
        let home_dir = dirs::home_dir().expect("Failed to get home directory");
        home_dir.join(".souce-cmd-gui-config.json")
    };
}

#[tauri::command]
async fn is_running(state: State<'_, Arc<Mutex<AppState>>>) -> Result<bool, ()> {
    Ok(state.lock().await.running_thread.is_some())
}

async fn load_or_create_config() -> Config {
    let config = Config::default();

    // Load config from file
    if let Ok(config_json) = tokio::fs::read_to_string(CONFIG_FILE.clone()).await {
        let config: Config = serde_json::from_str(&config_json).unwrap();
        return config;
    }

    // Save config to file as json
    let config_json = serde_json::to_string(&config).unwrap();

    tokio::fs::write(CONFIG_FILE.clone(), config_json)
        .await
        .expect("Failed to save config to file");

    info!("Saved config to file");

    config
}

#[tauri::command]
async fn get_config(state: State<'_, Arc<Mutex<AppState>>>) -> Result<Config, ()> {
    let state = state.lock().await;

    Ok(state.config.clone())
}

#[tauri::command]
async fn save_config(state: State<'_, Arc<Mutex<AppState>>>, config: Config) -> Result<(), ()> {
    let mut state = state.lock().await;

    state.config = config;

    // Save config to file as json
    let config_json = serde_json::to_string(&state.config).unwrap();

    tokio::fs::write(CONFIG_FILE.clone(), config_json)
        .await
        .expect("Failed to save config to file");

    info!("Saved config to file");

    Ok(())
}

#[tauri::command]
fn get_commands() -> Vec<CommandResponse> {
    commands::get_commands()
        .into_iter()
        .map(|command| command.into())
        .collect()
}

#[tauri::command]
async fn start(state: State<'_, Arc<Mutex<AppState>>>, config: Config) -> Result<(), ()> {
    let cloned_app_state = state.clone().inner().clone();
    let mut state = state.lock().await;

    if state.running_thread.is_some() {
        return Err(());
    }

    state.stop_flag = Arc::<AtomicBool>::default();

    let api_key = config.openai_api_key.clone();

    let cmd_state = CmdState {
        personality: String::new(),
        chat_gpt: ChatGPT::new(api_key).ok(),
        conversations: HashMap::new(),
        ollama: Ollama::default(),
        message_context: HashMap::new(),
        user_cooldowns: HashMap::new(),
    };

    state.cmd_state = cmd_state;

    let stop_flag = state.stop_flag.clone();

    let handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut builder = SourceCmdLogParser::builder()
                .file_path(Box::new(PathBuf::from(config.file_path)))
                .state(cloned_app_state)
                .set_parser(config.parser.get_parser())
                .stop_flag(stop_flag)
                .time_out(Duration::from_secs(config.command_timeout));

            for command in commands::get_commands() {
                if command.global_command {
                    builder = builder.add_global_command(move |msg, state| {
                        // Call the function in the trait object
                        command.command.call(msg, state)
                    });
                } else {
                    builder = builder.add_command(
                        &format!("{}", command.id),
                        move |msg, state| {
                            // Call the function in the trait object
                            command.command.call(msg, state)
                        },
                    );
                }
            }

            let mut parser = builder.build().expect("Failed to build parser");

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

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<Log>(100);

    logger::setup_logger(tx);

    let config = load_or_create_config().await;

    let app_state = AppState {
        running_thread: None,
        config,
        stop_flag: Arc::<AtomicBool>::default(),
        cmd_state: CmdState::default(),
    };

    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(app_state)))
        .invoke_handler(tauri::generate_handler![
            is_running,
            get_config,
            start,
            stop,
            get_commands,
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
