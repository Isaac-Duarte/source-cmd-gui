pub mod state;

use serde::{Deserialize, Serialize};
use source_cmd_parser::parsers::{CSSLogParser, Cs2LogParser};

#[derive(Clone, Serialize, Deserialize)]
pub enum GameParser {
    #[serde(rename = "Counter Strike 2")]
    CounterStrike2,

    #[serde(rename = "Counter Strike Source")]
    CounterStrikeSource,
}

impl GameParser {
    pub fn get_parser(&self) -> Box<dyn source_cmd_parser::log_parser::ParseLog> {
        match self {
            GameParser::CounterStrike2 => Box::<Cs2LogParser>::default(),
            GameParser::CounterStrikeSource => Box::<CSSLogParser>::default(),
        }
    }
}
