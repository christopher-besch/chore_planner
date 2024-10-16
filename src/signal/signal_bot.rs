use std::net::SocketAddr;

use crate::bot::MessagableBot;
use crate::bot::PollableBot;
use crate::bot::ReplyMsg;

use super::signal_client::RpcClient;
use super::transports;
use anyhow::Result;
use jsonrpsee::async_client::Client;
use jsonrpsee::async_client::ClientBuilder;
use serde_json::Value;

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
        let (sender, receiver) = transports::tcp::connect(self.endpoint.unwrap())
            .await
            .unwrap();
        let client = ClientBuilder::default().build_with_tokio(sender, receiver);

        SignalBot {
            client: client,
            group_id: self.group_id.clone().unwrap(),
            account_name: self.account_name.clone().unwrap(),
        }
    }
}

impl SignalBot {
    fn parse_update(&mut self, update: Value) -> Option<String> {
        // TODO: implement
        Some(format!("{update:#?}"))
    }
}

impl MessagableBot for SignalBot {
    async fn next_msg(&mut self) -> Option<String> {
        // TODO: remove unwrap
        // TODO: maybe move subscribe to constructor or builder
        let mut stream = self.client.subscribe_receive(None).await.unwrap();

        let update = stream.next().await;
        let msg = match update {
            Some(Ok(raw_msg)) => self.parse_update(raw_msg),
            Some(Err(e)) => {
                eprintln!("getting the next signal message failed: {:#}", e);
                None
            }
            None => None,
        };
        // TODO: remove unwrap
        stream.unsubscribe().await.unwrap();
        return msg;
    }

    async fn send_msg(&mut self, msg: Result<ReplyMsg>) {
        let text = msg.unwrap().mono_msg; // TODO
        let length = text.len();

        let result = self.client
            .send(
                None,
                vec![],
                vec![self.group_id.clone()],
                false,
                false,
                format!("{text}"),
                vec![],
                vec![],
                vec![format!("0:{length}:MONOSPACE")],
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
            .await;
    

        println!("Send: {:?}", result.is_ok());
    }

    fn get_name(&self) -> &str {
        self.account_name.as_str()
    }
}

/*
impl PollableBot for SignalBot {
    async fn send_poll(&mut self, question: &str, options: Vec<String>) -> Result<i32> {}
    
    async fn stop_poll(&mut self, poll_id: i32) -> Result<Vec<(String, u32)>> {}
}
*/
