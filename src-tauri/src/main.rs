// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod error;
mod lexer;
mod logger;
mod model;
mod python;
pub(crate) mod repository;

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
use error::{SourceCmdGuiError, SourceCmdGuiResult};
use lazy_static::lazy_static;
use log::{info, warn};
use logger::Log;
use model::{
    entity::Script,
    state::{AppState, CmdState, CommandResponse, Config},
};

use python::DynamicPythonCtx;
use repository::{JsonRepository, ScriptRepository};
use source_cmd_parser::log_parser::SourceCmdLogParser;
use tauri::{Manager, State};
use tokio::{
    fs,
    sync::{mpsc, Mutex},
};

lazy_static! {
    static ref CONFIG_DIR: PathBuf = {
        let home_dir = dirs::home_dir().expect("Failed to get home directory");

        home_dir.join(".source-cmd-gui/")
    };
    static ref CONFIG_FILE: PathBuf = CONFIG_DIR.join("config.json");
    static ref SCRIPTS_DIR: PathBuf = CONFIG_DIR.join("scripts");
    static ref SCRIPTS_REPOSITORY: PathBuf = SCRIPTS_DIR.join("repo.json");
}

#[tauri::command]
async fn is_running(state: State<'_, Arc<Mutex<AppState>>>) -> SourceCmdGuiResult<bool> {
    Ok(state.lock().await.running_thread.is_some())
}

async fn load_or_create_config() -> SourceCmdGuiResult<Config> {
    let config = Config::default();

    fs::create_dir_all(SCRIPTS_DIR.to_string_lossy().to_string()).await?;

    // Load config from file
    if let Ok(config_json) = tokio::fs::read_to_string(CONFIG_FILE.clone()).await {
        if let Ok(config) = serde_json::from_str::<Config>(&config_json) {
            return Ok(config);
        }

        return Ok(config);
    }

    // Save config to file as json
    let config_json = serde_json::to_string(&config).unwrap();

    tokio::fs::write(CONFIG_FILE.clone(), config_json).await?;

    info!("Saved config to file");

    Ok(config)
}

#[tauri::command]
async fn get_config(state: State<'_, Arc<Mutex<AppState>>>) -> SourceCmdGuiResult<Config> {
    let state = state.lock().await;

    Ok(state.config.clone())
}

#[tauri::command]
async fn get_code(
    state: State<'_, Arc<Mutex<AppState>>>,
    script_id: &str,
) -> SourceCmdGuiResult<String> {
    let state = state.lock().await;

    Ok(state
        .script_repository
        .get_script(script_id)?
        .get_code()
        .await?)
}

#[tauri::command]
async fn save_code(
    state: State<'_, Arc<Mutex<AppState>>>,
    script_id: &str,
    code: &str,
) -> SourceCmdGuiResult {
    let state = state.lock().await;

    state
        .script_repository
        .get_script(script_id)?
        .save_code(code)
        .await?;

    Ok(())
}

#[tauri::command]
async fn save_config(state: State<'_, Arc<Mutex<AppState>>>, config: Config) -> SourceCmdGuiResult {
    let mut state = state.lock().await;

    state.config = config;

    // Save config to file as json
    let config_json = serde_json::to_string(&state.config).unwrap();

    tokio::fs::write(CONFIG_FILE.clone(), config_json).await?;

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
async fn start(state: State<'_, Arc<Mutex<AppState>>>, config: Config) -> SourceCmdGuiResult {
    let cloned_app_state = state.clone().inner().clone();
    let mut state = state.lock().await;

    if state.running_thread.is_some() {
        return Err(SourceCmdGuiError::ProcessAlreadyRunning);
    }

    state.stop_flag.store(false, Ordering::Relaxed);

    let api_key = config.openai_api_key.clone();

    let cmd_state = CmdState {
        personality: String::new(),
        chat_gpt: ChatGPT::new(api_key).ok(),
        conversations: HashMap::new(),
        python_context: DynamicPythonCtx::default(),
    };

    state.cmd_state = cmd_state;

    let stop_flag = state.stop_flag.clone();

    let handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async move {
            let mut builder = SourceCmdLogParser::builder()
                .file_path(Box::new(PathBuf::from(config.file_path)))
                .state(cloned_app_state)
                .set_parser(config.parser.get_parser())
                .chat_key(config.parser.get_chat_key())
                .stop_flag(stop_flag)
                .owner(&config.owner)
                .time_out(Duration::from_secs(config.command_timeout));

            for command in commands::get_commands() {
                if command.global_command {
                    builder = builder.add_global_command(move |msg, state| {
                        // Call the function in the trait object
                        command.command.call(msg, state)
                    });
                } else {
                    builder = builder.add_command(&command.id.to_string(), move |msg, state| {
                        // Call the function in the trait object
                        command.command.call(msg, state)
                    });
                }
            }

            let mut parser = builder.build()?;

            parser.run().await
        });

        result.map_err(SourceCmdGuiError::SourceCmdParserError)
    });

    state.running_thread = Some(handle);

    Ok(())
}

#[tauri::command]
async fn stop(state: State<'_, Arc<Mutex<AppState>>>) -> SourceCmdGuiResult {
    let mut state = state.lock().await;
    if let Some(handle) = state.running_thread.take() {
        state.stop_flag.store(true, Ordering::Relaxed);
        handle.join().unwrap()?;
    }

    Ok(())
}

#[tauri::command]
async fn get_scripts(state: State<'_, Arc<Mutex<AppState>>>) -> SourceCmdGuiResult<Vec<Script>> {
    let state = state.lock().await;

    state.script_repository.get_scripts().await
}

#[tauri::command]
async fn add_script(
    state: State<'_, Arc<Mutex<AppState>>>,
    script_name: String,
) -> SourceCmdGuiResult<Script> {
    let mut state = state.lock().await;

    info!("Adding script: {:?}", script_name);

    state.script_repository.add_script(script_name).await
}

#[tauri::command]
async fn delete_script(state: State<'_, Arc<Mutex<AppState>>>, id: &str) -> SourceCmdGuiResult {
    let mut state = state.lock().await;

    state.script_repository.delete_script(id).await
}

#[tauri::command]
async fn update_script(
    state: State<'_, Arc<Mutex<AppState>>>,
    script: Script,
) -> SourceCmdGuiResult {
    let mut state = state.lock().await;

    state
        .script_repository
        .update_script(&script.id.clone(), script)
        .await
}

#[tokio::main]
async fn main() -> SourceCmdGuiResult {
    let (tx, mut rx) = mpsc::channel::<Log>(100);

    logger::setup_logger(tx);

    let config = load_or_create_config().await?;

    let mut app_state = AppState {
        running_thread: None,
        config,
        stop_flag: Arc::<AtomicBool>::default(),
        cmd_state: CmdState::default(),
        script_repository: JsonRepository::new(SCRIPTS_REPOSITORY.to_string_lossy().to_string())
            .await,
    };

    // Setup database tables
    app_state.script_repository.init().await?;

    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(app_state)))
        .invoke_handler(tauri::generate_handler![
            is_running,
            get_config,
            start,
            stop,
            get_commands,
            save_config,
            get_scripts,
            add_script,
            delete_script,
            update_script,
            get_code,
            save_code
        ])
        .setup(move |app| {
            let app_handle = app.handle();
            tauri::async_runtime::spawn(async move {
                while let Some(message) = rx.recv().await {
                    match app_handle.emit_all("stdout_data", &message) {
                        Ok(_) => {}
                        Err(_) => {
                            warn!("Failed to send stdout data to frontend");
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())?;

    Ok(())
}
