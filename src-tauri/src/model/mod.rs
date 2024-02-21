pub mod entity;
pub mod state;

use serde::{Deserialize, Serialize};
use source_cmd_parser::parsers::{CSSLogParser, Cs2LogParser};

use crate::commands::MinecraftParser;

#[derive(Clone, Serialize, Deserialize)]
pub enum GameParser {
    #[serde(rename = "Counter Strike 2")]
    CounterStrike2,

    #[serde(rename = "Counter Strike Source")]
    CounterStrikeSource,

    #[serde(rename = "Minecraft")]
    Minecraft,
}

impl GameParser {
    pub fn get_parser(&self) -> Box<dyn source_cmd_parser::log_parser::ParseLog> {
        match self {
            GameParser::CounterStrike2 => Box::<Cs2LogParser>::default(),
            GameParser::CounterStrikeSource => Box::<CSSLogParser>::default(),
            GameParser::Minecraft => Box::<MinecraftParser>::default(),
        }
    }

    pub fn get_chat_key(&self) -> enigo::Key {
        match self {
            GameParser::CounterStrike2 | GameParser::CounterStrikeSource => enigo::Key::Layout('y'),
            GameParser::Minecraft => enigo::Key::Layout('t'),
        }
    }
}
