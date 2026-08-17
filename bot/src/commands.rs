//! Telegram bot command definitions.

use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    Start,
    Help,
    Language,
    /// `/tara <SYMBOL>`
    Tara(String),
    /// `/alarm <SYMBOL> <THRESHOLD>`, space-separated.
    #[command(parse_with = "split")]
    Alarm { symbol: String, threshold: String },
}
