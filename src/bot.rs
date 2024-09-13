use anyhow::Result;
use std::collections::HashSet;
use std::ops::{Add, AddAssign};

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
}

/// a bot that supports creating polls
pub trait PollableBot {
    /// Create a new poll with a question and list of options.
    /// Return the identifier of this poll.
    async fn send_poll(&mut self, question: &str, options: Vec<String>) -> Result<i32>;
    /// Stop the specified poll and return a list of (option, count_chosen) tuples.
    async fn stop_poll(&mut self, poll_id: i32) -> Result<Vec<(String, u32)>>;
}
