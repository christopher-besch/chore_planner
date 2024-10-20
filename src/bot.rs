use anyhow::{bail, Error, Result};
use std::collections::HashSet;
use std::ops::{Add, AddAssign};
use std::str::FromStr;

/// a message the chat bot should write
#[derive(Debug, PartialEq)]
pub struct ReplyMsg {
    /// the message to send in monospace font
    pub mono_msg: String,
    /// the user chat tags to send
    pub tags: HashSet<String>,
}

impl Add for ReplyMsg {
    type Output = Self;

    fn add(mut self, rhs: ReplyMsg) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign for ReplyMsg {
    fn add_assign(&mut self, rhs: ReplyMsg) {
        if !self.mono_msg.is_empty() && !rhs.mono_msg.is_empty() {
            self.mono_msg += "\n\n\n\n";
        }
        self.mono_msg += &rhs.mono_msg;
        self.tags.extend(rhs.tags);
    }
}

impl ReplyMsg {
    pub fn new() -> Self {
        ReplyMsg {
            mono_msg: String::new(),
            tags: HashSet::new(),
        }
    }
    pub fn from_mono(mono_msg: &str) -> Self {
        ReplyMsg {
            mono_msg: mono_msg.to_string(),
            tags: HashSet::new(),
        }
    }
}

/// a bot that supports receiving and sending messages
pub trait MessagableBot {
    /// Wait for the next message intended for the bot
    /// (i.e. a message that starts with the bots name).
    /// This can be used in a loop.
    async fn next_msg(&mut self) -> Option<String>;
    /// Send a message or an error.
    async fn send_msg(&mut self, msg: Result<ReplyMsg>);
    /// Get the name of the bot i.e., the prefix of all accepted received messages.
    fn get_name(&self) -> &str;

    // This needs to be called once the bot isn't used any longer.
    //
    // This can't be implemented in the drop function as it is async.
    async fn shutdown(&mut self);
}

/// a bot that supports creating polls
pub trait PollableBot {
    /// Create a new poll with a question and list of options.
    /// Return the identifier of this poll.
    async fn send_poll(&mut self, question: &str, options: Vec<String>) -> Result<i64>;
    /// Stop the specified poll and return a list of (option, count_chosen) tuples.
    async fn stop_poll(&mut self, poll_id: i64) -> Result<Vec<(String, u32)>>;
}

/// the types of protocols the chore_planner supports in production
///
/// The TestBot is not made for production and thus not listed.
pub enum BotProtocol {
    Telegram,
    Signal,
}

impl FromStr for BotProtocol {
    type Err = Error;

    fn from_str(input: &str) -> Result<BotProtocol, Self::Err> {
        let lowercase: &str = &input.to_lowercase();
        match lowercase {
            "telegram" => Ok(BotProtocol::Telegram),
            "signal" => Ok(BotProtocol::Signal),
            _ => bail!("chat protocol '{lowercase}' is not supported"),
        }
    }
}
