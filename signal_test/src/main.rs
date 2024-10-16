mod transports;

use jsonrpsee::async_client::ClientBuilder;
use jsonrpsee::core::client::{Error as RpcError, Subscription, SubscriptionClientT};
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::proc_macros::rpc;
use serde_json::{Error, Value};
use std::net::SocketAddr;
use std::{path::PathBuf, time::Duration};
use tokio::{select, time::sleep};

/// Creates the type RpcClient
#[rpc(client)]
trait Rpc {
    #[allow(non_snake_case)]
    #[method(name = "send", param_kind = map)]
    fn send(
        &self,
        account: Option<String>,
        recipients: Vec<String>,
        groupIds: Vec<String>,
        noteToSelf: bool,
        endSession: bool,
        message: String,
        attachments: Vec<String>,
        mentions: Vec<String>,
        textStyle: Vec<String>,
        quoteTimestamp: Option<u64>,
        quoteAuthor: Option<String>,
        quoteMessage: Option<String>,
        quoteMention: Vec<String>,
        quoteTextStyle: Vec<String>,
        quoteAttachment: Vec<String>,
        preview_url: Option<String>,
        preview_title: Option<String>,
        preview_description: Option<String>,
        preview_image: Option<String>,
        sticker: Option<String>,
        storyTimestamp: Option<u64>,
        storyAuthor: Option<String>,
        editTimestamp: Option<u64>,
    ) -> Result<Value, ErrorObjectOwned>;

    #[subscription(
        name = "subscribeReceive" => "receive",
        unsubscribe = "unsubscribeReceive",
        item = Value,
        param_kind = map
    )]
    async fn subscribe_receive(&self, account: Option<String>) -> SubscriptionResult;
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let (sender, receiver) =
        transports::tcp::connect("127.0.0.1:42069".parse::<SocketAddr>().unwrap())
            .await
            .unwrap();

    let client = ClientBuilder::default().build_with_tokio(sender, receiver);

    // receive
    let mut stream = client.subscribe_receive(None).await.unwrap();

    {
        while let Some(v) = stream_next(30.0, &mut stream).await {
            let v = v.unwrap();
            println!("{v}");
            client.send(None, vec!["+4917695280753".to_string()], vec![], false, false, format!("{v}"), vec![], vec![], vec![], None, None, None, vec![], vec![], vec![], None, None, None, None, None, None, None, None).await;
        }
    }
    stream.unsubscribe().await.unwrap();
}

// TODO: don't do this!
async fn stream_next(
    timeout: f64,
    stream: &mut Subscription<Value>,
) -> Option<Result<Value, Error>> {
    if timeout < 0.0 {
        stream.next().await
    } else {
        select! {
            v = stream.next() => v,
            _= sleep(Duration::from_millis((timeout * 1000.0) as u64)) => None,
        }
    }
}
