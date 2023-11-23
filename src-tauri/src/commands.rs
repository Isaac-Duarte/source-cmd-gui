use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use chatgpt::types::CompletionResponse;
use log::{info, warn};
use ollama_rs::generation::completion::request::GenerationRequest;
use source_cmd_parser::{
    log_parser::{SouceError, SourceCmdFn},
    model::{ChatMessage, ChatResponse},
};
use tokio::sync::{Mutex, RwLock};

use crate::{
    lexer,
    model::state::{CmdState, UserCooldown, CommandResponse},
};
pub struct Command<T: Send + Sync + 'static> {
    pub command:  Box<dyn SourceCmdFn<T> + 'static>,
    pub name: String,
    pub description: String,
    pub global_command: bool,
}

impl<T: Send + Sync + 'static> Command<T> {
    fn new(command: Box<dyn SourceCmdFn<T> + 'static>, name: String, description: String, global_command: bool) -> Self {
        Self {
            command,
            name,
            description,
            global_command
        }
    }
}

impl From<Command<CmdState>> for CommandResponse {
    fn from(command: Command<CmdState>) -> Self {
        Self {
            enabled: true,
            name: command.name,
            description: command.description,
        }
    }
}

pub fn get_commands() -> Vec<Command<CmdState>> {
    vec![
        Command::new(Box::new(pong), "Ping".to_string(), "Pong!".to_string(), false),
        Command::new(
            Box::new(explain),
            "Explain".to_string(),
            "Generates a response from ChatGPT".to_string(),
            false
        ),
        Command::new(
            Box::new(personality),
            "Personality".to_string(),
            "Set the personality for ChatGPT".to_string(),
            false
        ),
        Command::new(
            Box::new(llama2),
            "Llama2".to_string(),
            "Generates a llama2 response (Requires Ollama)".to_string(),
            false 
        ),
        Command::new(
            Box::new(eval),
            "Eval".to_string(),
            "Evaluate a math expression".to_string(),
            true
        ),
    ]
}

async fn is_command_disabled(command_id: &str, disabled_commands: Arc<Mutex<Vec<String>>>) -> bool {
    disabled_commands
        .lock()
        .await
        .iter()
        .any(|command| command == command_id)
}

pub async fn pong(
    chat_message: ChatMessage,
    state: Arc<RwLock<CmdState>>,
) -> Result<Option<ChatResponse>, SouceError> {
    if is_command_disabled("Ping", state.read().await.disabled_commands.clone()).await {
        return Ok(None);
    }

    Ok(Some(ChatResponse::new(format!(
        "PONG {}",
        chat_message.message
    ))))
}

pub async fn explain(
    chat_message: ChatMessage,
    state: Arc<RwLock<CmdState>>,
) -> Result<Option<ChatResponse>, SouceError> {
    if is_command_disabled("Explain", state.read().await.disabled_commands.clone()).await {
        return Ok(None);
    }

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

pub async fn personality(
    chat_message: ChatMessage,
    state: Arc<RwLock<CmdState>>,
) -> Result<Option<ChatResponse>, SouceError> {
    if is_command_disabled("Persionality", state.read().await.disabled_commands.clone()).await {
        return Ok(None);
    }

    let message = chat_message.message;

    let mut state = state.write().await;

    state.personality = message;

    Ok(None)
}

pub async fn llama2(
    message: ChatMessage,
    state: Arc<RwLock<CmdState>>,
) -> Result<Option<ChatResponse>, SouceError> {
    if is_command_disabled("Llama2", state.read().await.disabled_commands.clone()).await {
        return Ok(None);
    }

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

pub async fn eval(
    chat_message: ChatMessage,
    state: Arc<RwLock<CmdState>>,
) -> Result<Option<ChatResponse>, SouceError> {
    if is_command_disabled("Eval", state.read().await.disabled_commands.clone()).await {
        return Ok(None);
    }

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
                let mut state = state.write().await;

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
