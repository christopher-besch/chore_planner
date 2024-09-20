use crate::{
    bot::{MessagableBot, PollableBot, ReplyMsg},
    db::Db,
    week::Week,
};

use anyhow::Result;
use chrono::Local;
use clap::{command, Parser, Subcommand};

#[derive(Parser)]
// TODO: author unused
// TODO: don't hard code line length
#[command(version, author, help_expected = true, term_width = 20)]
struct Cli {
    #[command(subcommand)]
    command: Option<MainCommand>,
}

#[derive(Subcommand)]
#[command(arg_required_else_help = true)]
enum MainCommand {
    /// change the planned worker
    ///
    /// the referenced tenant won't be considered for any chores in that week
    #[command(alias = "Replan")]
    Replan {
        /// the name of the tenant to exclude
        #[arg(long, alias = "Tenant")]
        tenant: String,

        /// the affected week
        #[arg(long, alias = "Week", value_parser = 1..54)]
        // this needs to be an i64 because of value_parser
        week: i64,

        /// the year of the affected week
        #[arg(long, alias = "Year")]
        year: i32,
    },
    /// create a report
    #[command(alias = "Report")]
    Report {
        /// first week to include in the report
        #[arg(long, alias = "Week", value_parser = 1..54)]
        // this needs to be an i64 because of value_parser
        week: i64,

        /// year of the first week to include in the report
        #[arg(long, alias = "Year")]
        year: i32,
    },
    /// administrate tenants
    #[command(alias = "Tenant")]
    Tenant {
        #[command(subcommand)]
        command: Option<TenantCommand>,
    },
    /// administrate chores
    #[command(alias = "Chore")]
    Chore {
        #[command(subcommand)]
        command: Option<ChoreCommand>,
    },
    /// administrate chore exemptions
    #[command(alias = "Exemption")]
    Exemption {
        #[command(subcommand)]
        command: Option<ExemptionCommand>,
    },
}

#[derive(Subcommand)]
#[command(arg_required_else_help = true)]
enum TenantCommand {
    /// list current tenants
    List,
    /// let a tenant move in
    ///
    /// fails when the room is not free
    ///
    /// fails when the tenant already occupies another room
    MoveIn {
        /// the unique name of the tenant
        #[arg(long, alias = "Name")]
        name: String,

        /// if the tenant has a unique chat tag (telegram doesn't enforce this) it can be used for
        /// tagging
        ///
        /// when the tenant already exists and you set this, the tag will be updated
        #[arg(long, alias = "Tag")]
        tag: Option<String>,

        /// the name of the room moving into
        #[arg(long, alias = "Room")]
        room: String,
    },
    /// let a tenant move out
    ///
    /// this vacates their current room and allows someone else to move in
    ///
    /// fails when the tenant is not a current tenant
    MoveOut {
        /// the name of the old tenant
        #[arg(long, alias = "Name")]
        name: String,
    },
    /// create a new room
    CreateRoom {
        /// the unique name of the new room
        #[arg(long, alias = "Name")]
        name: String,
    },
}

#[derive(Subcommand)]
#[command(arg_required_else_help = true)]
enum ChoreCommand {
    /// list all active chores
    List,
    /// create a new chore for all to enjoy
    Create {
        /// the unique name of the new chore
        #[arg(long, alias = "Name")]
        name: String,

        /// the job description of the new chore
        #[arg(long, alias = "Description")]
        description: String,
    },
    /// deactivate a chore
    ///
    /// this doesn't delete the chore but ignores it
    Deactivate {
        /// the name of the chore to deactivate
        #[arg(long, alias = "Name")]
        name: String,
    },
    /// reactivate a chore
    Reactivate {
        /// the name of the chore to reactivate
        #[arg(long, alias = "Name")]
        name: String,
    },
}

#[derive(Subcommand)]
#[command(arg_required_else_help = true)]
enum ExemptionCommand {
    /// list all chore exemptions
    List,
    /// create a new reason for exempting someone from a chore
    ///
    /// this doesn't already grant an exemption to a tenant, use the 'grant' subcommand for that
    Create {
        /// the name of the chores the exemption is for
        #[arg(long, alias = "Chores")]
        chores: Vec<String>,

        /// the unique reason for the exemption (i.e. `Müllminister`)
        #[arg(long, alias = "Reason")]
        reason: String,
    },
    /// redefine the set of chores the exemption is for
    ///
    /// the exemption needs to exist
    Change {
        /// the reason of the exemption (i.e. `Müllminister`)
        #[arg(long, alias = "Reason")]
        reason: String,

        /// the name of the chores the exemption is for
        #[arg(long, alias = "Chores")]
        chores: Vec<String>,
    },
    /// grant an exemption to a tenant
    ///
    /// the tenant may not have the exemption already
    Grant {
        /// the name of the tenant to receive the exemption
        #[arg(long, alias = "Tenant")]
        tenant: String,

        /// the reason of the exemption to grant (i.e. `Müllminister`)
        #[arg(long, alias = "Reason")]
        reason: String,
    },
    /// revoke an exemption from a tenant
    ///
    /// the tenant needs to have the to be revoked exemption
    Revoke {
        /// the unique reason for the exemption (i.e. `Müllminister`)
        #[arg(long, alias = "Reason")]
        reason: String,

        /// the name of the tenant to revoke the exemption from
        #[arg(long, alias = "Tenant")]
        tenant: String,
    },
}

/// Assigning a tenant to a ChoreLog prints a message instructing the tenant on how to mark
/// themselves as unwilling. This command is defined here and must be passed in many database
/// functions.
fn fmt_replan_cmd<B>(bot: &B) -> impl Fn(&str, Week) -> String
where
    B: MessagableBot,
{
    let bot_name = String::from(bot.get_name());
    move |tenant, week| {
        format!(
            "{} replan --tenant {} --week {} --year {}",
            bot_name,
            tenant,
            week.iso_week().week(),
            week.iso_week().year()
        )
    }
}

/// Parse a command string, perform the required action and return some response.
async fn run_command<F>(db: &mut Db, input: &str, fmt_replan_cmd: F) -> Result<ReplyMsg>
where
    F: Fn(&str, Week) -> String,
{
    println!("{}", input);
    let split_input = shellwords::split(input)?;
    // The help pages are also handled as errors and are thus send to the bot with this.
    let cli = Cli::try_parse_from(split_input)?;

    match &cli.command {
        Some(MainCommand::Tenant { command }) => match command {
            Some(TenantCommand::List) => db.list_tenants().await,
            Some(TenantCommand::MoveIn { name, tag, room }) => {
                db.move_in(name, tag, room, fmt_replan_cmd).await
            }
            Some(TenantCommand::MoveOut { name }) => db.move_out(name, fmt_replan_cmd).await,
            Some(TenantCommand::CreateRoom { name }) => db.create_room(name).await,
            None => panic!(),
        },
        Some(MainCommand::Chore { command }) => match command {
            Some(ChoreCommand::List) => db.list_plan(None).await,
            Some(ChoreCommand::Create { name, description }) => {
                db.create_chore(name, description, fmt_replan_cmd).await
            }
            Some(ChoreCommand::Deactivate { name }) => {
                db.set_chore_active_state(name, false, fmt_replan_cmd).await
            }
            Some(ChoreCommand::Reactivate { name }) => {
                db.set_chore_active_state(name, true, fmt_replan_cmd).await
            }
            None => panic!(),
        },
        Some(MainCommand::Exemption { command }) => match command {
            Some(ExemptionCommand::List) => db.list_exemptions().await,
            Some(ExemptionCommand::Create { reason, chores }) => {
                db.create_exemption_reason(reason, chores).await
            }
            Some(ExemptionCommand::Change { reason, chores }) => {
                db.change_exemption_reason(reason, chores, fmt_replan_cmd)
                    .await
            }
            Some(ExemptionCommand::Grant { reason, tenant }) => {
                db.grant_exemption(reason, tenant, fmt_replan_cmd).await
            }
            Some(ExemptionCommand::Revoke { reason, tenant }) => {
                db.revoke_exemption(reason, tenant, fmt_replan_cmd).await
            }
            None => panic!(),
        },
        Some(MainCommand::Replan { tenant, week, year }) => {
            db.replan(tenant, Week::new(*week as u32, *year)?, fmt_replan_cmd)
                .await
        }
        Some(MainCommand::Report { week, year }) => {
            db.print_report(Week::new(*week as u32, *year)?).await
        }
        None => panic!(),
    }
}

/// Perform the weekly action.
/// This function is idempotent, you can call it multiple times in the same week and nothing
/// happens. An exception to this is the database's debug mode.
pub async fn weekly_action<B: MessagableBot + PollableBot>(db: &mut Db, bot: &mut B) {
    println!("thanks for the SIGHUP; performing weekly action");
    let week_changed = db.set_week(Week::from(Local::now().date_naive()));
    if !week_changed {
        println!("the current week didn't change");
        return;
    }

    bot.send_msg(db.update_plan(fmt_replan_cmd(bot)).await)
        .await;

    if let Err(e) = db.stop_rating_polls(bot).await {
        eprintln!("Error stopping polls: {:?}", e);
    }
    if let Err(e) = db.create_rating_polls(bot).await {
        eprintln!("Error creating polls: {:?}", e);
    }

    bot.send_msg(db.print_next_week_banner().await).await;
}

/// Run a command and return the response to the bot.
pub async fn handle_next_msg<B: MessagableBot>(db: &mut Db, bot: &mut B, msg: &str) {
    bot.send_msg(run_command(db, msg, fmt_replan_cmd(bot)).await)
        .await;
}
