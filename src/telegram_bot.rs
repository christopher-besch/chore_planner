use crate::bot::{MessagableBot, PollableBot, ReplyMsg};

use anyhow::{bail, Result};
use futures_util::stream::StreamExt;
use std::pin::Pin;
use teloxide::{
    payloads::{SendMessageSetters, SendPollSetters},
    requests::Requester,
    types::{ChatId, MessageId, ParseMode, UpdateId},
    update_listeners::{self, AsUpdateStream, PollingStream},
    utils::markdown::code_block,
    Bot as TeloxideBot,
};
use tokio::time::{sleep, Duration};

/// Build a TelegramBot with this.
///
/// rust doesn't support self-referential structs. TelegramBot::update_stream references the
/// listerner. Therefore they need to be split into two structs.
///
/// The TelegramBotBuilder may not be dropped before the built TelegramBot.
pub struct TelegramBotBuilder {
    token: Option<String>,
    chat_id: Option<ChatId>,
    listener: Option<update_listeners::Polling<TeloxideBot>>,
}

pub struct TelegramBot<'a> {
    bot: TeloxideBot,
    chat_id: ChatId,
    /// references TelegramBotBuilder::listener
    update_stream: Pin<Box<PollingStream<'a, TeloxideBot>>>,
    bot_username: String,
    /// None when no update has been received yet
    last_id_received: Option<UpdateId>,
}

impl TelegramBotBuilder {
    pub fn new() -> Self {
        Self {
            token: None,
            chat_id: None,
            listener: None,
        }
    }

    /// Set the token the TelegramBot should use.
    pub fn token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }
    /// Set id of the chat the TelegramBot should listen on.
    pub fn chat_id(mut self, chat_id: ChatId) -> Self {
        self.chat_id = Some(chat_id);
        self
    }

    /// Build a TelegramBot.
    /// Don't drop the TelegramBotBuilder while using the bot.
    pub async fn build(&mut self) -> TelegramBot {
        let bot = TeloxideBot::new(self.token.as_ref().unwrap());
        self.listener = Some(update_listeners::polling_default(bot.clone()).await);
        let bot_username = format!("@{}", bot.get_me().await.unwrap().username.clone().unwrap());
        TelegramBot {
            bot,
            chat_id: self.chat_id.unwrap(),
            update_stream: Box::pin(self.listener.as_mut().unwrap().as_stream()),
            bot_username,
            last_id_received: None,
        }
    }
}

impl<'a> TelegramBot<'a> {
    /// Check if the update is a message to hand to the application.
    fn parse_update(&mut self, update: teloxide::types::Update) -> Option<String> {
        if let Some(last_id_received) = self.last_id_received {
            if update.id <= last_id_received {
                println!("ignore old id: {}", update.id.0);
                return None;
            }
        }
        self.last_id_received = Some(update.id);
        match update.kind {
            teloxide::types::UpdateKind::Message(msg) => {
                if msg.chat.id != self.chat_id {
                    eprintln!("ignoring new chat with id: {}", msg.chat.id);
                    return None;
                }
                match msg.text() {
                    Some(text) => {
                        // ignore messages not meant for this bot
                        if text
                            .to_string()
                            .trim_start()
                            .to_lowercase()
                            .starts_with(&self.bot_username)
                        {
                            Some(text.to_string())
                        } else {
                            eprintln!("ignore as it doesn't start with {}", self.bot_username);
                            None
                        }
                    }
                    None => None,
                }
            }
            _ => None,
        }
    }

    /// Split big messages into multiple.
    fn paginate_mono_msg(msg: &str) -> Vec<String> {
        // telegrams limit is 4096 but let's leave some padding
        const MSG_LIMIT: usize = 4050;

        let lines = msg.split('\n');
        let mut paged_mono: Vec<String> = vec![];
        for line in lines {
            // Can we still fit the new line into the last message?
            if let Some(last_paged_mono) = paged_mono.last_mut() {
                if last_paged_mono.len() + 1 + line.len() <= MSG_LIMIT {
                    last_paged_mono.push('\n');
                    last_paged_mono.push_str(line);
                    continue;
                }
            }
            // Can the line fit into its own message?
            if line.len() <= MSG_LIMIT {
                paged_mono.push(line.to_string());
                continue;
            }
            // The line must be split into multiple messages.
            eprintln!("Error: ignored too long line");

            // TODO: split into multiple messages
            // This code is not utf-8 safe and might panic.
            // let mut cur_line = line.to_string();
            // while !cur_line.is_empty() {
            //     let (chunk, rest) = cur_line.split_at(std::cmp::min(cur_line.len(), MSG_LIMIT));
            //     paged_mono.push(chunk.to_string());
            //     cur_line = rest.to_string();
            // }
        }
        paged_mono
    }
}

impl<'a> MessagableBot for TelegramBot<'a> {
    async fn next_msg(&mut self) -> Option<String> {
        let update_res = self.update_stream.next().await;
        match update_res {
            Some(Ok(update)) => self.parse_update(update),
            Some(Err(e)) => {
                eprintln!("getting the next telegram update failed: {:#}", e);
                None
            }
            None => None,
        }
    }

    async fn send_msg(&mut self, msg: Result<ReplyMsg>) {
        const TIME_BETWEEN_MESSAGES: Duration = Duration::from_millis(500);

        let msg = msg.unwrap_or_else(|e| {
            eprintln!("sending error: {:?}", e);
            ReplyMsg::from_mono(&e.to_string())
        });

        let mut paginated_mono_msgs = Self::paginate_mono_msg(&msg.mono_msg)
            .into_iter()
            .peekable();
        while let Some(paginated_mono_msg) = paginated_mono_msgs.next() {
            let paginated_mono_msg_trimmed = paginated_mono_msg.trim();
            // ignore empty messages
            if paginated_mono_msg_trimmed.is_empty() {
                continue;
            }
            if let Err(e) = <TeloxideBot as Requester>::send_message(
                &self.bot,
                self.chat_id,
                code_block(paginated_mono_msg_trimmed),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .await
            {
                eprintln!("Error sending mono message: {:?}", e);
            };
            println!("sent message");
            // wait between sending messages
            if paginated_mono_msgs.peek().is_some() {
                sleep(TIME_BETWEEN_MESSAGES).await;
            }
        }

        if !msg.tags.is_empty() {
            sleep(TIME_BETWEEN_MESSAGES).await;
            if let Err(e) = <TeloxideBot as Requester>::send_message(
                &self.bot,
                self.chat_id,
                msg.tags
                    .clone()
                    .into_iter()
                    .collect::<Vec<String>>()
                    .join(" "),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .await
            {
                eprintln!("Error sending tags {:?}: {:?}", msg.tags, e);
            };
        }
    }

    fn get_name(&self) -> &str {
        &self.bot_username
    }
}

impl<'a> PollableBot for TelegramBot<'a> {
    async fn send_poll(&mut self, question: &str, options: Vec<String>) -> Result<i32> {
        let msg = <TeloxideBot as Requester>::send_poll(&self.bot, self.chat_id, question, options)
            .allows_multiple_answers(false)
            .is_anonymous(true)
            .await?;
        println!("created poll {}", msg.id.0);
        Ok(msg.id.0)
    }

    async fn stop_poll(&mut self, poll_id: i32) -> Result<Vec<(String, u32)>> {
        let poll =
            <TeloxideBot as Requester>::stop_poll(&self.bot, self.chat_id, MessageId(poll_id))
                .await?;
        if !poll.is_closed {
            bail!("the poll failed to close");
        }
        println!("closed poll {}", poll_id);

        Ok(poll
            .options
            .into_iter()
            .map(|option| (option.text, option.voter_count))
            .collect())
    }
}
