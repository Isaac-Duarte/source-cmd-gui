use std::{sync::Arc, time::Duration};

use chatgpt::types::CompletionResponse;
use log::info;
use source_cmd_parser::{
    log_parser::{ParseLog, SourceCmdFn},
    model::{ChatMessage, ChatResponse},
};
use tokio::sync::Mutex;

use crate::{
    error::SourceCmdGuiError,
    lexer,
    model::state::{AppState, CommandResponse},
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

    tokio::time::sleep(Duration::from_secs(10)).await;

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
    let (chat_gpt, personality) = {
        let state = state.lock().await;

        (
            state.cmd_state.chat_gpt.clone(),
            state.cmd_state.personality.clone(),
        )
    };

    if let Some(chat_gpt) = chat_gpt {
        let response: CompletionResponse = chat_gpt
        .send_message(format!(
            "Please response in 120 characters or less. Can you response as if you were {}. The prompt is: \"{}\"",
            personality,
            chat_message.message
        ))
        .await?;

        let mut personality = personality;

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

    match meval::eval_str(expression.replace('x', "*")) {
        Ok(response) => {
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

    return Ok(Some(ChatResponse::new(
        chat_message.raw_message.to_string(),
    )));
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
    if !can_run_command("python", &state).await {
        return Ok(None);
    }

    let message = chat_message.raw_message.clone();

    // Get the first word of the message
    let command = message.split_whitespace().next().unwrap_or_default();

    let (script, config, python_context) = {
        let state = state.lock().await;

        (
            state
                .script_repository
                .get_script_by_trigger(command)
                .await
                .ok()
                .flatten(),
            state.config.clone(),
            state.cmd_state.python_context.clone(),
        )
    };

    if let Some(script) = script {
        chat_message.command = command.to_string();
        chat_message.raw_message = chat_message
            .raw_message
            .replace(command, "")
            .trim()
            .to_string();
        chat_message.message = message.replace(command, "").trim().to_string();

        let (response, context) =
            python::process_python_command(&script, chat_message, &config, python_context).await?;

        {
            let mut state = state.lock().await;

            if let Some(context) = context {
                state.cmd_state.python_context = context;
            }
        }

        Ok(response)
    } else {
        Ok(None)
    }
}

pub struct MinecraftParser {
    regex: regex::Regex,
}

impl MinecraftParser {
    pub fn new() -> Self {
        Self {
            regex: regex::Regex::new(r"\[CHAT\].*\] (.+?): (.*)").unwrap(),
        }
    }
}

impl Default for MinecraftParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ParseLog for MinecraftParser {
    fn parse_command(&self, line: &str) -> Option<ChatMessage> {
        if let Some(captures) = self.regex.captures(line) {
            let user_name = captures.get(1).unwrap().as_str().to_string();
            let message = captures.get(2).unwrap().as_str().to_string();

            let command = message.split_whitespace().next().unwrap().to_string();
            let raw_message = message.clone();

            let message = if message.starts_with(command.as_str()) {
                message[command.len()..].trim().to_string()
            } else {
                message
            };

            Some(ChatMessage::new(user_name, message, command, raw_message))
        } else {
            None
        }
    }
}
