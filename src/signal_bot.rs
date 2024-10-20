mod signal_cli_interface;

use crate::{
    bot::{MessagableBot, PollableBot, ReplyMsg},
    paginate::paginate_str,
    signal_bot::{signal_cli_interface::tcp, signal_cli_interface::RpcClient},
};

use anyhow::{bail, Context, Result};
use jsonrpsee::{async_client::Client, async_client::ClientBuilder, core::client::Subscription};
use serde::Deserialize;
use serde_json::Value;
use std::{net::SocketAddr, time::Duration};
use tokio::time::sleep;

pub struct SignalBotBuilder {
    endpoint: Option<SocketAddr>,
    group_id: Option<String>,
    account_name: Option<String>,
    display_name: Option<String>,
    allow_message_from_self: Option<bool>,
}

pub struct SignalBot {
    client: Client,
    group_id: String,
    account_name: String,
    display_name: String,
    allow_message_from_self: bool,

    // None when destructed already
    receive_stream: Option<Subscription<Value>>,
}

impl SignalBotBuilder {
    pub fn new() -> Self {
        Self {
            endpoint: None,
            group_id: None,
            account_name: None,
            display_name: None,
            allow_message_from_self: None,
        }
    }
    pub fn endpoint(mut self, endpoint: SocketAddr) -> SignalBotBuilder {
        self.endpoint = Some(endpoint);
        self
    }
    pub fn group_id(mut self, group_id: String) -> SignalBotBuilder {
        self.group_id = Some(group_id);
        self
    }
    pub fn account_name(mut self, account_name: String) -> SignalBotBuilder {
        self.account_name = Some(account_name);
        self
    }
    pub fn display_name(mut self, display_name: String) -> SignalBotBuilder {
        self.display_name = Some(display_name);
        self
    }
    pub fn allow_message_from_self(
        &mut self,
        allow_message_from_self: bool,
    ) -> &mut SignalBotBuilder {
        self.allow_message_from_self = Some(allow_message_from_self);
        self
    }
    pub async fn build(&mut self) -> SignalBot {
        let (sender, receiver) = tcp::connect(self.endpoint.unwrap()).await.expect("Error: tcp connection to signal-cli failed; maybe start it with something like 'signal-cli daemon --tcp 127.0.0.1:42069'");
        let client = ClientBuilder::default().build_with_tokio(sender, receiver);
        let receive_stream = Some(client.subscribe_receive(None).await.unwrap());

        SignalBot {
            client,
            receive_stream,
            group_id: self.group_id.clone().unwrap(),
            account_name: self.account_name.clone().unwrap(),
            display_name: self.display_name.clone().unwrap(),
            allow_message_from_self: self.allow_message_from_self.unwrap(),
        }
    }
}

impl SignalBot {
    /// Check if the update is a message to hand to the application.
    fn parse_update(&mut self, update: Value) -> Option<String> {
        // example message from a sender that isn't the bot
        // Object {
        //     "account": String("+491717171717"),
        //     "envelope": Object {
        //         "dataMessage": Object {
        //             "expiresInSeconds": Number(0),
        //             "groupInfo": Object {
        //                 "groupId": String("Wbvq4+oxG9b+RY619QbRMLyffm4pPOTqmMJJlOWYoYs="),
        //                 "type": String("DELIVER"),
        //             },
        //             "mentions": Array [
        //                 Object {
        //                     "length": Number(1),
        //                     "name": String("+491717171717"),
        //                     "number": String("+491717171717"),
        //                     "start": Number(0),
        //                     "uuid": String("d53a76a6-b318-f4865e69b774"),
        //                 },
        //             ],
        //             "message": String("￼  some message"),
        //             "timestamp": Number(1729171912889),
        //             "viewOnce": Bool(false),
        //         },
        //         "source": String("+491717171717"),
        //         "sourceDevice": Number(1),
        //         "sourceName": String("Adam Jensen"),
        //         "sourceNumber": String("+491717181818"),
        //         "sourceUuid": String("a5f5acf8-ab9e-669d2d93dbfc"),
        //         "timestamp": Number(1729171912889),
        //     },
        // }
        #[derive(Deserialize, Debug)]
        struct GroupInfo {
            #[serde(rename = "groupId")]
            group_id: String,
        }
        #[derive(Deserialize, Debug)]
        struct Mention {
            name: Option<String>,
            number: Option<String>,
        }
        #[derive(Deserialize, Debug)]
        struct SentMessage {
            #[serde(rename = "groupInfo")]
            group_info: GroupInfo,
            message: String,
            mentions: Vec<Mention>,
        }
        #[derive(Deserialize, Debug)]
        struct SyncMessage {
            #[serde(rename = "sentMessage")]
            sent_message: SentMessage,
        }
        #[derive(Deserialize, Debug)]
        struct Envelope {
            #[serde(rename = "sourceNumber")]
            source_number: String,
            // syncMessage when message is from self, dataMessage when from someone else
            #[serde(rename = "syncMessage")]
            sync_message: Option<SyncMessage>,
            #[serde(rename = "dataMessage")]
            data_message: Option<SentMessage>,
        }
        #[derive(Deserialize, Debug)]
        struct Update {
            account: String,
            envelope: Envelope,
        }

        // TODO: remove debug print
        println!("{:#?}", update);
        match serde_json::from_value::<Update>(update.clone()) {
            Ok(update) => {
                // get sent_message from message sent by bot or from someone else
                let sent_message = match update.envelope.sync_message {
                    Some(sync_message) => sync_message.sent_message,
                    None => match update.envelope.data_message {
                        Some(data_message) => data_message,
                        None => {
                            println!("this update isn't a sent message");
                            return None;
                        }
                    },
                };
                if update.account != self.account_name {
                    eprintln!(
                        "ignoring message meant for account name: {}",
                        update.account
                    );
                    return None;
                }
                if !self.allow_message_from_self
                    && update.envelope.source_number == self.account_name
                {
                    eprintln!(
                        "ignoring message from bot account: {}",
                        update.envelope.source_number
                    );
                    return None;
                }
                if !sent_message.mentions.into_iter().any(|m| {
                    m.number == Some(self.account_name.clone())
                        || m.name == Some(self.account_name.clone())
                }) {
                    eprintln!("ignoring message that doesn't mention the bot");
                    return None;
                }
                if sent_message.group_info.group_id != self.group_id {
                    eprintln!(
                        "ignoring message from new group with group_id: {}",
                        sent_message.group_info.group_id
                    );
                    return None;
                }
                // The first word is a special character representing the @chore_planner_bot mention.
                // This needs to be replaced with the literal @chore_planner_bot string.
                match sent_message.message.find(" ") {
                    Some(pos) => {
                        let mut cmd_message = sent_message.message;
                        cmd_message.replace_range(0..pos, &self.display_name);
                        Some(cmd_message)
                    }
                    None => Some(self.display_name.clone()),
                }
            }
            Err(e) => {
                println!("failed to parse update, probably to be ignored: {e:?}\n{update:#?}");
                None
            }
        }
    }

    /// send a signal message formatted in monospace font
    async fn send_mono_str(&self, msg: &str) -> Result<i64> {
        let length = msg.len();
        let format = vec![format!("0:{length}:MONOSPACE")];
        self.send_raw_str(msg, format).await
    }
    /// send a signal message with some format
    async fn send_raw_str(&self, msg: &str, format: Vec<String>) -> Result<i64> {
        let result = self
            .client
            .send(
                vec![],
                vec![self.group_id.clone()],
                msg.to_string(),
                vec![],
                vec![],
                format,
            )
            .await?;

        #[derive(Deserialize, Debug)]
        struct SendResult {
            #[serde(rename = "type")]
            type_field: String,
        }
        #[derive(Deserialize, Debug)]
        struct SendReturn {
            results: Vec<SendResult>,
            timestamp: i64,
        }

        let send_return =
            serde_json::from_value::<SendReturn>(result).context("Sending Message failed")?;
        match send_return
            .results
            .iter()
            .all(|r| r.type_field == "SUCCESS")
        {
            true => Ok(send_return.timestamp),
            false => bail!("send_return isn't all success: {send_return:#?}"),
        }
    }
}

impl MessagableBot for SignalBot {
    async fn next_msg(&mut self) -> Option<String> {
        // The stream is opened at start. When it is closed here, the chore_planner can no longer
        // function and needs to be restarted.
        let stream = self.receive_stream.as_mut().unwrap();
        let update = stream.next().await;
        match update {
            Some(Ok(raw_msg)) => self.parse_update(raw_msg),
            Some(Err(e)) => {
                eprintln!("getting the next signal message failed: {:#}", e);
                None
            }
            None => None,
        }
    }

    async fn send_msg(&mut self, msg: Result<ReplyMsg>) {
        const TIME_BETWEEN_MESSAGES: Duration = Duration::from_millis(1000);
        // signal doesn't appear to have a limit but too long messages need to be unfolded
        const MSG_LIMIT: usize = 2000;

        let msg = msg.unwrap_or_else(|e| {
            eprintln!("sending error: {:?}", e);
            ReplyMsg::from_mono(&e.to_string())
        });
        let mut paginated_mono_msgs = paginate_str(&msg.mono_msg, MSG_LIMIT)
            .into_iter()
            .peekable();
        while let Some(paginated_mono_msg) = paginated_mono_msgs.next() {
            let paginated_mono_msg_trimmed = paginated_mono_msg.trim();
            // ignore empty messages
            if paginated_mono_msg_trimmed.is_empty() {
                continue;
            }
            if let Err(e) = self.send_mono_str(paginated_mono_msg_trimmed).await {
                eprintln!("Error sending mono message: {:?}", e);
            };
            println!("sent message");
            // wait between sending messages
            if paginated_mono_msgs.peek().is_some() {
                sleep(TIME_BETWEEN_MESSAGES).await;
            }
        }
        // TODO: implement tags
    }

    fn get_name(&self) -> &str {
        self.display_name.as_str()
    }

    // this can't be implemented in the drop function as it is async
    async fn shutdown(&mut self) {
        // We're shutting down the chore_planner now anyways.
        let stream = Option::take(&mut self.receive_stream).unwrap();
        stream.unsubscribe().await.unwrap();
        println!("closed Signal receive stream");
    }
}

impl PollableBot for SignalBot {
    /// The question and options may not be longer than some 2000 bytes combined.
    async fn send_poll(&mut self, question: &str, options: Vec<String>) -> Result<i64> {
        let msg = question.to_string()
            + "\n\n"
            + &options.into_iter().collect::<Vec<String>>().join("\n");

        self.send_mono_str(&msg).await
    }

    async fn stop_poll(&mut self, _poll_id: i64) -> Result<Vec<(String, u32)>> {
        // TODO: implement persistent storage of emoji reactions
        // this appears to require persistent storage, requiring changing the database scheme
        bail!("stopping polls isn't implemented for Signal")
    }
}
