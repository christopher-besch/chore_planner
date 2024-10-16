use jsonrpsee::proc_macros::rpc;
use serde_json::Value;

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
