use futures_util::{stream::StreamExt, Sink, SinkExt, Stream};
use jsonrpsee::core::{
    async_trait,
    client::{ReceivedMessage, TransportReceiverT, TransportSenderT},
};
use jsonrpsee::proc_macros::rpc;
use serde_json::Value;
use thiserror::Error;

#[path = "./stream_codec.rs"]
mod stream_codec;
#[path = "./tcp.rs"]
pub mod tcp;

#[derive(Debug, Error)]
enum Errors {
    #[error("Other: {0}")]
    Other(String),
    #[error("Closed")]
    Closed,
}

struct Sender<T: Send + Sink<String>> {
    inner: T,
}

#[async_trait]
impl<T: Send + Sink<String, Error = impl std::error::Error> + Unpin + 'static> TransportSenderT
    for Sender<T>
{
    type Error = Errors;

    async fn send(&mut self, body: String) -> Result<(), Self::Error> {
        self.inner
            .send(body)
            .await
            .map_err(|e| Errors::Other(format!("{:?}", e)))?;
        Ok(())
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        self.inner
            .close()
            .await
            .map_err(|e| Errors::Other(format!("{:?}", e)))?;
        Ok(())
    }
}

struct Receiver<T: Send + Stream> {
    inner: T,
}

#[async_trait]
impl<T: Send + Stream<Item = Result<String, std::io::Error>> + Unpin + 'static> TransportReceiverT
    for Receiver<T>
{
    type Error = Errors;

    async fn receive(&mut self) -> Result<ReceivedMessage, Self::Error> {
        match self.inner.next().await {
            None => Err(Errors::Closed),
            Some(Ok(msg)) => Ok(ReceivedMessage::Text(msg)),
            Some(Err(e)) => Err(Errors::Other(format!("{:?}", e))),
        }
    }
}

/// Creates the struct RpcClient
#[rpc(client)]
trait Rpc {
    #[allow(non_snake_case)]
    #[method(name = "send", param_kind = map)]
    fn send(
        &self,
        recipients: Vec<String>,
        groupIds: Vec<String>,
        message: String,
        attachments: Vec<String>,
        mentions: Vec<String>,
        textStyle: Vec<String>,
    ) -> Result<Value, ErrorObjectOwned>;

    #[subscription(
        name = "subscribeReceive" => "receive",
        unsubscribe = "unsubscribeReceive",
        item = Value,
        param_kind = map
    )]
    async fn subscribe_receive(&self, account: Option<String>) -> SubscriptionResult;
}
