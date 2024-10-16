use std::net::SocketAddr;

use crate::bot::MessagableBot;
use crate::bot::PollableBot;
use crate::bot::ReplyMsg;

use super::signal_client::RpcClient;
use super::transports;
use anyhow::Result;
use jsonrpsee::async_client::Client;
use jsonrpsee::async_client::ClientBuilder;

pub struct SignalBotBuilder {
    endpoint: Option<SocketAddr>,
    group_id: Option<String>,
    do_register: Option<bool>,
    do_link: Option<bool>,
    account_name: Option<String>,
}

pub struct SignalBot {
    client: Client,
    account_name: String,
}

impl SignalBotBuilder {
    pub fn new() -> Self {
        Self {
            endpoint: None,
            group_id: None,
            do_register: None,
            do_link: None,
            account_name: None,
        }
    }

    pub fn endpoint(&mut self, endpoint: SocketAddr) -> &mut SignalBotBuilder {
        self.endpoint = Some(endpoint);
        return self;
    }

    pub async fn build(&mut self) -> SignalBot {
        let (sender, receiver) = transports::tcp::connect(self.endpoint.unwrap())
            .await
            .unwrap();
        let client = ClientBuilder::default().build_with_tokio(sender, receiver);
        let name = self.account_name.as_ref().unwrap();

        SignalBot {
            client: client,
            account_name: name,
        }
    }
}

impl MessagableBot for SignalBot {
    async fn next_msg(&mut self) -> Option<String> {
        let mut stream = self.client.subscribe_receive(None);
        let msg = stream.next().await;
        stream.unsubscribe().await.unwrap();

        return msg;
    }

    async fn send_msg(&mut self, msg: Result<ReplyMsg>) {
        let text = msg.unwrap().mono_msg; // TODO

        self.client
            .send(
                None,
                vec![],
                vec![],
                false,
                false,
                format!("{text}"),
                vec![],
                vec![],
                vec![],
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
    }

    fn get_name(&self) -> &str {
        ""
    }
}

impl PollableBot for SignalBot {
    async fn send_poll(&mut self, question: &str, options: Vec<String>) -> Result<i32> {}

    async fn stop_poll(&mut self, poll_id: i32) -> Result<Vec<(String, u32)>> {}
}
