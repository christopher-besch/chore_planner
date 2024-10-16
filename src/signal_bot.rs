mod signal_cli_interface;

use crate::bot::MessagableBot;
use crate::bot::PollableBot;
use crate::bot::ReplyMsg;

use crate::paginate::paginate_str;
use crate::signal_bot::signal_cli_interface::tcp;
use crate::signal_bot::signal_cli_interface::RpcClient;

use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use jsonrpsee::async_client::Client;
use jsonrpsee::async_client::ClientBuilder;
use jsonrpsee::core::client::Subscription;
use serde::Deserialize;
use serde_json::Value;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::sleep;

pub struct SignalBotBuilder {
    endpoint: Option<SocketAddr>,
    group_id: Option<String>,
    account_name: Option<String>,
    do_register: bool,
    do_link: bool,
}

pub struct SignalBot {
    client: Client,
    group_id: String,
    account_name: String,

    // None when destructed already
    receive_stream: Option<Subscription<Value>>,
}

impl SignalBotBuilder {
    pub fn new() -> Self {
        Self {
            endpoint: None,
            group_id: None,
            account_name: None,
            do_register: false,
            do_link: false,
        }
    }

    pub fn endpoint(&mut self, endpoint: SocketAddr) -> &mut SignalBotBuilder {
        self.endpoint = Some(endpoint);
        return self;
    }

    pub fn group_id(&mut self, group_id: String) -> &mut SignalBotBuilder {
        self.group_id = Some(group_id);
        return self;
    }

    pub fn account_name(&mut self, account_name: String) -> &mut SignalBotBuilder {
        self.account_name = Some(account_name);
        return self;
    }

    pub fn do_register(&mut self, do_register: bool) -> &mut SignalBotBuilder {
        self.do_register = do_register;
        return self;
    }

    pub fn do_link(&mut self, do_link: bool) -> &mut SignalBotBuilder {
        self.do_link = do_link;
        return self;
    }

    pub async fn build(&mut self) -> SignalBot {
        let (sender, receiver) = tcp::connect(self.endpoint.unwrap()).await.unwrap();
        let client = ClientBuilder::default().build_with_tokio(sender, receiver);
        let receive_stream = Some(client.subscribe_receive(None).await.unwrap());

        SignalBot {
            client,
            receive_stream,
            group_id: self.group_id.clone().unwrap(),
            account_name: self.account_name.clone().unwrap(),
        }
    }
}

impl SignalBot {
    fn parse_update(&mut self, update: Value) -> Option<String> {
        #[derive(Deserialize, Debug)]
        struct GroupInfo {
            #[serde(rename = "groupId")]
            group_id: String,
        }
        #[derive(Deserialize, Debug)]
        struct SentMessage {
            #[serde(rename = "groupInfo")]
            group_info: GroupInfo,
            message: String,
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

        match serde_json::from_value::<Update>(update.clone()) {
            Ok(update) => {
                let sent_message = match update.envelope.sync_message {
                    Some(sync_message) => sync_message.sent_message,
                    None => match update.envelope.data_message {
                        Some(data_message) => data_message,
                        None => return None,
                    },
                };
                // TODO: enable this again after testing is done
                // if update.account != self.account_name {
                //     return None;
                // }
                // if update.envelope.source_number == self.account_name {
                //     return None;
                // }
                if sent_message.group_info.group_id != self.group_id {
                    return None;
                }
                Some(sent_message.message)
            }
            Err(e) => {
                println!("Warning: {e:?}\n{update:#?}");
                None
            }
        }
    }

    async fn send_mono_str(&self, msg: &str) -> Result<i64> {
        let length = msg.len();
        let format = vec![format!("0:{length}:MONOSPACE")];
        self.send_raw_str(msg, format).await
    }
    async fn send_unformatted_str(&self, msg: &str) -> Result<i64> {
        self.send_raw_str(msg, vec![]).await
    }
    async fn send_raw_str(&self, msg: &str, format: Vec<String>) -> Result<i64> {
        // may the rust gods have mercy with this API
        let result = self
            .client
            .send(
                None,
                vec![],
                vec![self.group_id.clone()],
                false,
                false,
                msg.to_string(),
                vec![],
                vec![],
                format,
                None,
                None,
                None,
                vec![],
                vec![],
                vec![],
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
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
        const TIME_BETWEEN_MESSAGES: Duration = Duration::from_millis(500);
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

        if !msg.tags.is_empty() {
            sleep(TIME_BETWEEN_MESSAGES).await;
            if let Err(e) = self
                .send_unformatted_str(
                    &msg.tags
                        .clone()
                        .into_iter()
                        .collect::<Vec<String>>()
                        .join(" "),
                )
                .await
            {
                eprintln!("Error sending tags {:?}: {:?}", msg.tags, e);
            };
        }
    }

    fn get_name(&self) -> &str {
        // TODO: use human readable name, not telephone number
        self.account_name.as_str()
    }

    // this can't be implemented in the drop function as it is async
    async fn shutdown(&mut self) {
        let stream = std::mem::replace(&mut self.receive_stream, None).unwrap();
        stream.unsubscribe().await.unwrap();
        println!("closed Signal receive stream");
    }
}

impl PollableBot for SignalBot {
    /// the question and options may not be longer than some 2000 bytes combined
    async fn send_poll(&mut self, question: &str, options: Vec<String>) -> Result<i64> {
        let msg = question.to_string()
            + "\n\n"
            + &options.into_iter().collect::<Vec<String>>().join("\n");

        self.send_mono_str(&msg).await
    }

    async fn stop_poll(&mut self, poll_id: i64) -> Result<Vec<(String, u32)>> {
        // this appears to require persistent storage, requiring changing the database scheme
        bail!("stopping polls isn't implemented for Signal")
    }
}
