use crate::{bot::PollableBot, db::*};

use anyhow::Context;

pub const RATING_OPTIONS: [&str; 5] = [
    "1  😢  You didn't do anything.",
    "2  👎  Barely noticeable.",
    "3  😮  Acceptable.",
    "4  👍  Well done.",
    "5  ❤️  Perfect!",
];

impl Db {
    /// Create a poll for all ChoreLogs of last week.
    ///
    /// The poll is only created once. Any subsequent calls are being ignored.
    pub async fn create_rating_polls<T: PollableBot>(&mut self, bot: &mut T) -> Result<()> {
        let week = Week::from_db(self.get_week_internal().await.db_week() - 1);
        let rows = sqlx::query(
            r#"
SELECT Chore.name, Tenant.name
FROM ChoreLog
JOIN Tenant
    ON Tenant.id = ChoreLog.worker
JOIN Chore
    ON Chore.id = ChoreLog.chore_id
WHERE ChoreLog.week = ?1
AND ChoreLog.rating_poll_id IS NULL;
"#,
        )
        .bind(week.db_week())
        .fetch_all(&mut self.con)
        .await?;
        self.integrity_check().await?;

        for row in rows {
            let chore: String = row.try_get(0)?;
            let tenant: String = row.try_get(1)?;

            let id = bot
                .send_poll(
                    &format!("How well did {} do the {} on {}?", tenant, chore, week),
                    RATING_OPTIONS.iter().map(|r| r.to_string()).collect(),
                )
                .await?;

            let affected_rows = sqlx::query(
                r#"
UPDATE ChoreLog
SET rating_poll_id = ?1
    WHERE ChoreLog.week = ?2
    AND ChoreLog.chore_id = (SELECT Chore.id FROM Chore WHERE Chore.name = ?3);
"#,
            )
            .bind(id)
            .bind(week.db_week())
            .bind(chore)
            .execute(&mut self.con)
            .await?
            .rows_affected();
            self.integrity_check().await?;
            if affected_rows != 1 {
                bail!("affected {} rows", affected_rows);
            }
        }
        Ok(())
    }

    /// Stop all open polls and store their results in the database.
    pub async fn stop_rating_polls<T: PollableBot>(&mut self, bot: &mut T) -> Result<()> {
        let rows = sqlx::query(
            r#"
SELECT ChoreLog.rating_poll_id
FROM ChoreLog
    WHERE ChoreLog.completed = 0
    AND ChoreLog.rating_poll_id IS NOT NULL
    -- only stop polls before this week
    AND ChoreLog.week < ?1;
"#,
        )
        .bind(self.get_week_internal().await.db_week())
        .fetch_all(&mut self.con)
        .await?;
        self.integrity_check().await?;
        let poll_ids = rows
            .into_iter()
            .map(|r| -> Result<i64> { Ok(r.try_get(0)?) })
            .collect::<Result<Vec<i64>>>()?;

        for poll_id in poll_ids {
            let results = bot.stop_poll(poll_id).await.unwrap_or_else(|e| {
                eprintln!(
                    "stopping poll failed, mark the poll {} as completed anyways, this can happen when the bot is moved to a different chat: {}",
                    poll_id, e
                );
                vec![]
            });
            for (rating_str, count) in results {
                // Figure out what to store in the db.
                let rating = rating_str
                    .split(' ')
                    .next()
                    .context("failed to parse rating_str")?
                    .parse::<u32>()?;
                // Store that count times.
                for _ in 0..count {
                    let affected_rows = sqlx::query(
                        r#"
INSERT INTO Rating VALUES (
    NULL,
    (SELECT ChoreLog.chore_id FROM ChoreLog WHERE ChoreLog.rating_poll_id = ?1),
    (SELECT ChoreLog.week     FROM ChoreLog WHERE ChoreLog.rating_poll_id = ?1),
    ?2
);
"#,
                    )
                    .bind(poll_id)
                    .bind(rating)
                    .execute(&mut self.con)
                    .await?
                    .rows_affected();
                    self.integrity_check().await?;
                    if affected_rows != 1 {
                        bail!("affected {} rows", affected_rows);
                    }
                }
            }
            let affected_rows = sqlx::query(
                r#"
UPDATE ChoreLog
SET completed = 1
    WHERE ChoreLog.rating_poll_id = ?1;
"#,
            )
            .bind(poll_id)
            .execute(&mut self.con)
            .await?
            .rows_affected();
            self.integrity_check().await?;
            if affected_rows != 1 {
                bail!("affected {} rows", affected_rows);
            }
        }
        Ok(())
    }
}
