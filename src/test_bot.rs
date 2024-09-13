use crate::bot::{MessagableBot, PollableBot, ReplyMsg};

use anyhow::Result;

/// a bot for integration testing the application
pub struct TestBot<
    StringIterator: Iterator<Item = String>,
    ReplyMsgIterator: Iterator<Item = Result<ReplyMsg>>,
> {
    /// the messages the bot should send the application
    pub to_send_msgs: StringIterator,
    /// the messages the bot expects the application to return
    pub expected_msgs: ReplyMsgIterator,
    /// the polls the bot expects the application to create
    ///
    /// use index as poll id
    /// needs to have same order as to_send_polls
    pub expected_polls: Vec<(String, Vec<String>)>,
    /// the poll results the bot should return
    ///
    /// use index as poll id
    /// needs to have same order as expected_polls
    pub to_send_polls: Vec<Vec<(String, u32)>>,
    /// the id of the next poll to create
    pub next_poll_id: usize,
}

impl<
        StringIterator: Iterator<Item = String>,
        ReplyMsgIterator: Iterator<Item = Result<ReplyMsg>>,
    > MessagableBot for TestBot<StringIterator, ReplyMsgIterator>
{
    async fn next_msg(&mut self) -> Option<String> {
        self.to_send_msgs.next()
    }

    async fn send_msg(&mut self, msg: Result<ReplyMsg>) {
        let expected = self.expected_msgs.next().unwrap();

        match msg {
            Ok(msg) => {
                if let Ok(expected) = expected {
                    assert_eq!(msg, expected);
                } else {
                    panic!();
                }
            }
            Err(msg) => {
                if let Err(expected) = expected {
                    assert_eq!(msg.to_string(), expected.to_string());
                } else {
                    panic!();
                }
            }
        }
    }

    fn get_name(&self) -> &str {
        "hihi_im_a_test"
    }
}

impl<
        StringIterator: Iterator<Item = String>,
        ReplyMsgIterator: Iterator<Item = Result<ReplyMsg>>,
    > PollableBot for TestBot<StringIterator, ReplyMsgIterator>
{
    async fn send_poll(&mut self, question: &str, options: Vec<String>) -> Result<i32> {
        let t = self.expected_polls[self.next_poll_id].clone();
        assert_eq!(t, (question.to_string(), options));
        let poll_id = self.next_poll_id;
        self.next_poll_id += 1;
        Ok(poll_id as i32)
    }

    async fn stop_poll(&mut self, poll_id: i32) -> Result<Vec<(String, u32)>> {
        Ok(self.to_send_polls[poll_id as usize].clone())
    }
}
