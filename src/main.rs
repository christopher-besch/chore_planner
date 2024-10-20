mod bot;
mod command;
mod db;
mod paginate;
mod signal_bot;
mod telegram_bot;
mod test_bot;
mod week;

use crate::{
    bot::MessagableBot, bot::PollableBot, db::Db, signal_bot::SignalBotBuilder,
    telegram_bot::TelegramBotBuilder, week::Week,
};

use anyhow::Context;
use bot::BotProtocol;
use chrono::Local;
use command::{handle_next_msg, weekly_action};
use std::{env, net::ToSocketAddrs, time::Duration};
use teloxide::types::ChatId;
use tokio::{
    signal::unix::{signal, SignalKind},
    time::sleep,
};

/// This is the main loop the application runs.
async fn run_loop<T: MessagableBot + PollableBot>(mut db: Db, mut bot: T) {
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
    bot.shutdown().await;
}

/// Create the database and bot before starting the main application loop.
///
/// The bot can't be created by a different function and then passed over as the TelegramBotBuilder may not be
/// dropped.
async fn initialize_and_run() {
    let bot_protocol = env::var("CHORE_PLANNER_CHAT_PROTOCOL")
        .expect("the environment variable CHORE_PLANNER_CHAT_PROTOCOL must be provided")
        .parse::<BotProtocol>()
        .context("failed to convert CHORE_PLANNER_CHAT_PROTOCOL to BotProtocol")
        .unwrap();
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
    let try_exclude_busy_tenants = env::var("CHORE_PLANNER_TRY_EXCLUDE_BUSY_TENANTS")
        .expect("the environment variable CHORE_PLANNER_TRY_EXCLUDE_BUSY_TENANTS must be provided")
        .parse::<bool>()
        .context("failed to convert CHORE_PLANNER_TRY_EXCLUDE_BUSY_TENANTS to bool")
        .unwrap();
    let db_path = env::var("CHORE_PLANNER_DB_PATH")
        .expect("the environment variable CHORE_PLANNER_DB_PATH must be provided");
    let fallback_to_last_week = env::var("CHORE_PLANNER_FALLBACK_TO_LAST_WEEK")
        .expect("the environment variable CHORE_PLANNER_FALLBACK_TO_LAST_WEEK must be provided")
        .parse::<bool>()
        .context("failed to convert CHORE_PLANNER_FALLBACK_TO_LAST_WEEK to bool")
        .unwrap();

    let mut fallback_week = Week::from(Local::now().date_naive());
    if fallback_to_last_week {
        fallback_week = Week::from_db(fallback_week.db_week() - 1);
    }
    let db = Db::new(
        &format!("sqlite://{}", db_path),
        fallback_week,
        weeks_to_plan,
        gamma,
        try_exclude_busy_tenants,
        rand::random::<u64>(),
        debug,
    )
    .await
    .unwrap();

    match bot_protocol {
        BotProtocol::Telegram => {
            println!("Creating a Telegram bot");
            let telegram_bot_token = env::var("TELEGRAM_BOT_TOKEN")
                .expect("the environment variable TELEGRAM_BOT_TOKEN must be provided");
            let telegram_chat_id = ChatId(
                env::var("TELEGRAM_CHAT_ID")
                    .unwrap_or_else(|_| {
                        eprintln!("the environment variable TELEGRAM_CHAT_ID should be provided");
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
            run_loop(db, bot).await;
        }
        BotProtocol::Signal => {
            println!("Creating a Signal bot");
            let endpoint = env::var("SIGNAL_CLI_ENDPOINT")
                .expect("the environment variable SIGNAL_CLI_ENDPOINT must be provided")
                .to_socket_addrs()
                .context("failed to convert SIGNAL_CLI_ENDPOINT to SocketAddr")
                .unwrap()
                .next()
                .context("failed to find any ip address for SIGNAL_CLI_ENDPOINT via DNS")
                .unwrap();
            println!("using endpoint: {endpoint}");
            let group_id = env::var("SIGNAL_GROUP_ID")
                .expect("the environment variable SIGNAL_GROUP_ID must be provided");
            let account_name = env::var("SIGNAL_ACCOUNT_NAME")
                .expect("the environment variable SIGNAL_ACCOUNT_NAME must be provided");
            let display_name = env::var("SIGNAL_DISPLAY_NAME")
                .expect("the environment variable SIGNAL_DISPLAY_NAME must be provided");
            let allow_message_from_self = env::var("SIGNAL_ALLOW_MESSAGE_FROM_SELF")
                .expect("the environment variable SIGNAL_ALLOW_MESSAGE_FROM_SELF must be provided")
                .parse::<bool>()
                .context("failed to convert SIGNAL_ALLOW_MESSAGE_FROM_SELF to bool")
                .unwrap();

            println!("waiting 10sec to let signal-cli boot up");
            sleep(Duration::from_secs(10)).await;
            let bot = SignalBotBuilder::new()
                .account_name(account_name)
                .display_name(display_name)
                .endpoint(endpoint)
                .group_id(group_id)
                .allow_message_from_self(allow_message_from_self)
                .build()
                .await;
            run_loop(db, bot).await;
        }
    }
}

#[tokio::main]
async fn main() {
    // TODO: ASCII art splash screen
    initialize_and_run().await;
}
