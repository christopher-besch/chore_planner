mod bot;
mod command;
mod db;
mod telegram_bot;
mod test_bot;
mod week;

use crate::{
    bot::MessagableBot, bot::PollableBot, db::Db, telegram_bot::TelegramBotBuilder, week::Week,
};

use anyhow::Context;
use chrono::Local;
use command::{handle_next_msg, weekly_action};
use std::env;
use teloxide::types::ChatId;
use tokio::signal::unix::{signal, SignalKind};

/// This is the main loop the application runs.
async fn run_loop<T: MessagableBot + PollableBot>(mut bot: T) {
    let debug = env::var("CHORE_PLANNER_DEBUG")
        .expect("the environment variable CHORE_PLANNER_DEBUG must be provided")
        .parse::<bool>()
        .context("failed to convert CHORE_PLANNER_DEBUG to bool")
        .unwrap();
    let weeks_to_plan = env::var("CHORE_PLANNER_WEEKS_TO_PLAN")
        .expect("the environment variable CHORE_PLANNER_WEEKS_TO_PLAN must be provided")
        .parse::<u32>()
        .context("failed to convert CHORE_PLANNER_WEEKS_TO_PLAN to u32")
        .unwrap();
    let gamma = env::var("CHORE_PLANNER_GAMMA")
        .expect("the environment variable CHORE_PLANNER_GAMMA must be provided")
        .parse::<f64>()
        .context("failed to convert CHORE_PLANNER_GAMMA to f64")
        .unwrap();

    let mut db = Db::new(
        "sqlite://chore_planner.sqlite",
        Week::from(Local::now().date_naive()),
        weeks_to_plan,
        gamma,
        rand::random::<u64>(),
        debug,
    )
    .await
    .unwrap();

    let mut sighup_stream = signal(SignalKind::hangup()).unwrap();
    let mut sigint_stream = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm_stream = signal(SignalKind::terminate()).unwrap();

    println!("waiting for bot updates");
    loop {
        tokio::select! {
            _ = sigint_stream.recv() => {
                break;
            }
            _ = sigterm_stream.recv() => {
                break;
            }
            _ = sighup_stream.recv() => {
                weekly_action(&mut db, &mut bot).await;
            }
            msg_opt = bot.next_msg() => {
                if let Some(msg) = msg_opt {
                    handle_next_msg(&mut db, &mut bot, &msg).await;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let telegram_bot_token = env::var("TELEGRAM_BOT_TOKEN")
        .expect("the environment variable TELEGRAM_BOT_TOKEN must be provided");
    let telegram_chat_id = ChatId(
        env::var("TELEGRAM_CHAT_ID")
            .unwrap_or_else(|_| {
                eprintln!("the environment varialbe TELEGRAM_CHAT_ID should be provided");
                "0".to_string()
            })
            .parse::<i64>()
            .expect("failed to convert environment variable TELEGRAM_CHAT_ID to i64"),
    );
    // the builder may not be deleted as the bot holds a borrow of it
    let mut bot_builder = TelegramBotBuilder::new()
        .token(telegram_bot_token)
        .chat_id(telegram_chat_id);
    let bot = bot_builder.build().await;
    run_loop(bot).await;
}
