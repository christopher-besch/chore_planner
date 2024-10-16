use crate::bot::{MessagableBot, PollableBot, ReplyMsg};

use anyhow::{bail, Result};

pub struct SignalBotBuilder {}

pub struct SignalBot {}

impl MessagableBot for SignalBot {
    async fn next_msg(&mut self) -> Option<String> {
        None
    }

    async fn send_msg(&mut self, msg: Result<ReplyMsg>) {}

    fn get_name(&self) -> &str {
        "implement"
    }
}

impl PollableBot for SignalBot {
    async fn send_poll(&mut self, question: &str, options: Vec<String>) -> Result<i32> {
        bail!("implement");
    }

    async fn stop_poll(&mut self, poll_id: i32) -> Result<Vec<(String, u32)>> {
        bail!("implement");
    }
}
