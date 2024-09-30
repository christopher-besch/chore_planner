// backend helper functions
mod exemption;
mod key_value;
mod plan;
mod scheme;
mod tenant;

// front end interface with command system
pub mod chore_commands;
pub mod exemption_commands;
pub mod plan_commands;
pub mod rating;
pub mod report_commands;
pub mod tenant_commands;

use crate::{bot::ReplyMsg, week::Week};

use anyhow::{bail, Result};
use rand::{rngs::StdRng, SeedableRng};
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, Row, SqliteConnection};
use std::str::FromStr;

#[cfg(test)]
#[path = "./tests/db_test.rs"]
mod db_test;

/// the central application databse
/// The entire state is stored here, therefore restarting the application is fine.
pub struct Db {
    con: SqliteConnection,
    /// When the current_week global variable hasn't been set, this is used instead.
    /// The global variable is set at the first weekly action.
    fallback_week: Week,
    /// how many weeks to plan ahead and thus create ChoreLogs for
    weeks_to_plan: u32,
    /// probability distribution parameter between in [0, 1]
    /// The higher gamma the more extremely the score effects the tenants probability of being
    /// chosen for a chore.
    /// 0 results the tenant with the highest score to never be picked (unless all tenants have the
    ///   same score).
    /// 1 results in all tenants having the same probability regardless of their score.
    ///
    /// See the mathematical proof in the repo.
    gamma: f64,
    rng: StdRng,
    // Increase the week every time a SIGHUP is received.
    debug: bool,
}

impl Db {
    /// Create a new database or load a database from some path.
    /// When the debug mode is on, advance to the next week every time the week is updated.
    pub async fn new(
        path: &str,
        fallback_week: Week,
        weeks_to_plan: u32,
        gamma: f64,
        seed: u64,
        debug: bool,
    ) -> Result<Self> {
        if !(0.0..=1.0).contains(&gamma) {
            bail!("gamma needs to be in [0, 1]");
        }
        if debug {
            eprintln!("Warning: debug mode is enabled!");
        }
        let mut db = Db {
            con: SqliteConnectOptions::from_str(path)?
                .foreign_keys(true)
                .create_if_missing(true)
                .connect()
                .await?,
            fallback_week,
            weeks_to_plan,
            gamma,
            rng: StdRng::seed_from_u64(seed),
            debug,
        };
        db.migrate().await?;
        db.integrity_check().await?;
        Ok(db)
    }

    /// Update the current week.
    /// Ignore the provided week when debug mode is enabled. In that case the current week is
    /// simply incremented.
    ///
    /// Return true iff the new week differs from the old
    pub async fn set_week(&mut self, week: Week) -> bool {
        let old_week = self.get_week_internal().await;
        if self.debug {
            self.set_week_internal(Week::from_db(old_week.db_week() + 1))
                .await;
        } else {
            self.set_week_internal(week).await;
        }
        let new_week = self.get_week_internal().await;
        println!("the current week is: {}", new_week);
        old_week != new_week
    }

    /// Typing on mobile complicated things with auto-correcting keyboards.
    /// Ignore the tenants capitalization to make things easier.
    fn capitalize_tenant_name(name: &str) -> String {
        let mut out: Vec<String> = vec![];
        for part in name.split(' ') {
            let mut chars = part.chars();
            // see: https://stackoverflow.com/questions/38406793/why-is-capitalizing-the-first-letter-of-a-string-so-convoluted-in-rust
            out.push(match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
            });
        }
        out.join(" ")
    }
}
