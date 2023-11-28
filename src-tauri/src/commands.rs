use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use chatgpt::types::CompletionResponse;
use log::{info, warn};
use ollama_rs::generation::completion::request::GenerationRequest;
use source_cmd_parser::{
    log_parser::SourceCmdFn,
    model::{ChatMessage, ChatResponse},
};
use tokio::sync::Mutex;

use crate::{
    error::SourceCmdGuiError,
    lexer,
    model::state::{AppState, CommandResponse, UserCooldown},
    python,
    repository::ScriptRepository,
};
pub struct Command<
    T: Unpin + Clone + Send + Sync + 'static,
    E: std::error::Error + Send + Sync + 'static,
> {
    pub command: Box<dyn SourceCmdFn<T, E> + 'static>,
    pub name: String,
    pub id: String,
    pub description: String,
    pub global_command: bool,
}

impl<T: Unpin + Clone + Send + Sync + 'static, E: std::error::Error + Send + Sync + 'static>
    Command<T, E>
{
    fn new(
        command: Box<dyn SourceCmdFn<T, E> + 'static>,
        name: String,
        id: String,
        description: String,
        global_command: bool,
    ) -> Self {
        Self {
            command,
            name,
            id,
            description,
            global_command,
        }
    }
}

impl From<Command<Arc<Mutex<AppState>>, SourceCmdGuiError>> for CommandResponse {
    fn from(command: Command<Arc<Mutex<AppState>>, SourceCmdGuiError>) -> Self {
        Self {
            enabled: true,
            name: command.name,
            description: command.description,
            id: command.id,
        }
    }
}

async fn can_run_command(command: &str, state: &Arc<Mutex<AppState>>) -> bool {
    let state = state.lock().await;

    if state
        .config
        .disabled_commands
        .contains(&command.to_string())
    {
        return false;
    }

    true
}

pub fn get_commands() -> Vec<Command<Arc<Mutex<AppState>>, SourceCmdGuiError>> {
    vec![
        Command::new(
            Box::new(pong),
            "Ping".to_string(),
            ".ping".to_string(),
            "Pong!".to_string(),
            false,
        ),
        Command::new(
            Box::new(explain),
            "Explain".to_string(),
            ".explain".to_string(),
            "Generates a response from ChatGPT".to_string(),
            false,
        ),
        Command::new(
            Box::new(personality),
            "Personality".to_string(),
            ".personality".to_string(),
            "Set the personality for ChatGPT".to_string(),
            false,
        ),
        Command::new(
            Box::new(llama2),
            "Llama2".to_string(),
            ".llama2".to_string(),
            "Generates a llama2 response (Requires Ollama)".to_string(),
            false,
        ),
        Command::new(
            Box::new(eval),
            "Eval".to_string(),
            "eval".to_string(),
            "Evaluate a math expression".to_string(),
            true,
        ),
        Command::new(
            Box::new(chat_gpt_respond),
            "ChatGPT Respond".to_string(),
            "chatgpt".to_string(),
            "Generates a response from ChatGPT for every message sent in chat.".to_string(),
            true,
        ),
        Command::new(
            Box::new(logger),
            "Logger".to_string(),
            "logger".to_string(),
            "Logs names into console".to_string(),
            true,
        ),
        Command::new(
            Box::new(mimic),
            "Mimic".to_string(),
            "mimic".to_string(),
            "Mimics the message sent".to_string(),
            true,
        ),
        Command::new(
            Box::new(handle_python_execution),
            "Python".to_string(),
            "python".to_string(),
            "Executes python code, disabling will disable all python commands".to_string(),
            true,
        ),
    ]
}

pub async fn pong(
    chat_message: ChatMessage,
    state: Arc<Mutex<AppState>>,
) -> Result<Option<ChatResponse>, SourceCmdGuiError> {
    if !can_run_command(&chat_message.command, &state).await {
        return Ok(None);
    }

    Ok(Some(ChatResponse::new(format!(
        "PONG {}",
        chat_message.message
    ))))
}

pub async fn explain(
    chat_message: ChatMessage,
    state: Arc<Mutex<AppState>>,
) -> Result<Option<ChatResponse>, SourceCmdGuiError> {
    if !can_run_command(&chat_message.command, &state).await {
        return Ok(None);
    }

    info!("Explain: {}", chat_message.message);
    let state = state.lock().await;

    if let Some(chat_gpt) = state.cmd_state.chat_gpt.as_ref() {
        let mut personality = state.cmd_state.personality.clone();

        let response: CompletionResponse = chat_gpt
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
    } else {
        Ok(None)
    }
}

pub async fn personality(
    chat_message: ChatMessage,
    state: Arc<Mutex<AppState>>,
) -> Result<Option<ChatResponse>, SourceCmdGuiError> {
    if !can_run_command(&chat_message.command, &state).await {
        return Ok(None);
    }

    let message = chat_message.message;

    let mut state = state.lock().await;

    state.cmd_state.personality = message;

    Ok(None)
}

pub async fn llama2(
    chat_message: ChatMessage,
    state: Arc<Mutex<AppState>>,
) -> Result<Option<ChatResponse>, SourceCmdGuiError> {
    if !can_run_command(&chat_message.command, &state).await {
        return Ok(None);
    }

    let mut state = state.lock().await;

    let mut request = GenerationRequest::new(
        "llama2-uncensored:latest".to_string(),
        format!(
            "Please keep the response under 120 characters. {} Says \"{}\"",
            chat_message.user_name, chat_message.message
        ),
    );

    if let Some(context) = state.cmd_state.message_context.get(&chat_message.user_name) {
        request = request.context(context.clone());
    }

    let response = state.cmd_state.ollama.generate(request).await;

    if let Ok(response) = response {
        state.cmd_state.message_context.insert(
            chat_message.user_name.clone(),
            response.final_data.unwrap().context,
        );
        Ok(Some(ChatResponse::new(response.response)))
    } else {
        Ok(None)
    }
}

const COOLDOWN_DURATION: Duration = Duration::from_secs(120); // 2 minutes
const MESSAGE_LIMIT: usize = 50;

pub async fn eval(
    chat_message: ChatMessage,
    state: Arc<Mutex<AppState>>,
) -> Result<Option<ChatResponse>, SourceCmdGuiError> {
    if !can_run_command("eval", &state).await {
        return Ok(None);
    }

    let message = chat_message.raw_message;

    if message.trim().parse::<f64>().is_ok() {
        return Ok(None);
    }

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
                let mut state = state.lock().await;

                let user_cooldown = state
                    .cmd_state
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

            info!("Eval: {} = {}", message, response);
            Ok(Some(ChatResponse::new(response.to_string())))
        }
        Err(_) => Ok(None),
    }
}

async fn chat_gpt_respond(
    chat_message: ChatMessage,
    state: Arc<Mutex<AppState>>,
) -> Result<Option<ChatResponse>, SourceCmdGuiError> {
    if !can_run_command("chatgpt", &state).await {
        return Ok(None);
    }

    let message = chat_message.raw_message;

    let mut state = state.lock().await;
    let response_direction = state.config.response_direction.clone();
    let user_name = &state.config.owner;

    if chat_message.user_name.contains(user_name) || chat_message.message.starts_with('.') {
        return Ok(None);
    }

    if let Some(chat_gpt) = state.cmd_state.chat_gpt.clone() {
        let conversation = state
            .cmd_state
            .conversations
            .entry(chat_message.user_name.clone())
            .or_insert_with(|| chat_gpt.new_conversation_directed(response_direction));

        let response: CompletionResponse = conversation
            .send_message(format!("{} says: \"{}\"", chat_message.user_name, message))
            .await?;

        let chat_response = response.message_choices[0].message.content.clone();

        Ok(Some(ChatResponse::new(chat_response.to_string())))
    } else {
        Ok(None)
    }
}

async fn logger(
    chat_message: ChatMessage,
    state: Arc<Mutex<AppState>>,
) -> Result<Option<ChatResponse>, SourceCmdGuiError> {
    if !can_run_command("logger", &state).await {
        return Ok(None);
    }

    info!(
        "{} said {}",
        chat_message.user_name, chat_message.raw_message
    );

    Ok(None)
}

async fn mimic(
    chat_message: ChatMessage,
    state: Arc<Mutex<AppState>>,
) -> Result<Option<ChatResponse>, SourceCmdGuiError> {
    if !can_run_command("mimic", &state).await {
        return Ok(None);
    }

    let owner = {
        let state = state.lock().await;

        state.config.owner.clone()
    };

    if chat_message.user_name.contains(&owner) {
        return Ok(None);
    }

    let message = chat_message.raw_message;

    Ok(Some(ChatResponse::new(message)))
}

/// Handles python execution
///
/// # Arguments
/// chat_message - The chat message
/// state - The app state
///
/// # Returns
/// A chat response if the command was executed
async fn handle_python_execution(
    mut chat_message: ChatMessage,
    state: Arc<Mutex<AppState>>,
) -> Result<Option<ChatResponse>, SourceCmdGuiError> {
    let message = chat_message.raw_message.clone();

    // Get the first word of the message
    let command = message.split_whitespace().next().unwrap_or_default();

    let state = state.lock().await;

    if let Some(script) = state
        .script_repository
        .get_script_by_trigger(command)
        .await
        .ok()
        .flatten()
    {
        chat_message.command = command.to_string();
        chat_message.raw_message = chat_message
            .raw_message
            .replace(command, "")
            .trim()
            .to_string();
        chat_message.message = message.replace(command, "").trim().to_string();

        let response = python::process_python_command(&script, chat_message, &state.config);

        Ok(response)
    } else {
        Ok(None)
    }
}
