use crate::db::*;

use anyhow::Result;
use sqlx::Row;
use tabled::{
    settings::{object::Segment, Alignment, Settings},
    Table, Tabled,
};

impl Db {
    /// Mark a tenant unwilling for some week.
    /// They won't be selected for a ChorePlan.
    pub async fn replan<F>(
        &mut self,
        tenant: &str,
        week: Week,
        fmt_replan_cmd: F,
    ) -> Result<ReplyMsg>
    where
        F: Fn(&str, Week) -> String,
    {
        let tenant = Self::capitalize_tenant_name(tenant);
        let affected_rows = sqlx::query(
            r#"
REPLACE INTO Unwilling VALUES (
    (SELECT Tenant.id FROM Tenant WHERE Tenant.name = ?1), ?2
)
"#,
        )
        .bind(tenant)
        .bind(week.db_week())
        .execute(&mut self.con)
        .await?
        .rows_affected();
        self.integrity_check().await?;
        if affected_rows != 1 {
            bail!("affected {} rows", affected_rows);
        }
        self.update_plan(fmt_replan_cmd).await
    }

    /// List all future ChoreLogs.
    ///
    /// Or list all past ChoreLogs starting from start_week when start_week is Some.
    pub async fn list_plan(&mut self, start_week: Option<Week>) -> Result<ReplyMsg> {
        let cur_week = self.get_week_internal().await;

        struct Chore {
            id: i32,
            name: String,
            description: String,
            times_performed: i32,
        }
        let chore_rows = sqlx::query(
            r#"
SELECT Chore.id, Chore.name, Chore.description, COUNT(ChoreLog.chore_id)
FROM Chore
LEFT JOIN ChoreLog
    ON Chore.id = ChoreLog.chore_id
    AND ChoreLog.week <= ?1
WHERE Chore.active = 1
GROUP BY Chore.id, Chore.name, Chore.description, Chore.active
ORDER BY Chore.id;
"#,
        )
        .bind(cur_week.db_week())
        .fetch_all(&mut self.con)
        .await?;
        self.integrity_check().await?;
        let chores = chore_rows
            .into_iter()
            .map(|r| {
                Ok(Chore {
                    id: r.try_get(0)?,
                    name: r.try_get(1)?,
                    description: r.try_get(2)?,
                    times_performed: r.try_get::<i32, usize>(3)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let mut out_mono = String::new();
        for chore in chores {
            #[derive(Tabled)]
            struct ChoreLogRow {
                week: String,
                tenant: String,
                rating: String,
            }
            let chore_log_rows = sqlx::query(
                r#"
SELECT ChoreLog.week, Tenant.name, CAST(AVG(Rating.rating) AS FLOAT)
FROM ChoreLog
JOIN Tenant
    ON Tenant.id = ChoreLog.worker
JOIN Chore
    ON Chore.id = ChoreLog.chore_id
LEFT JOIN Rating
    ON Rating.for_chore_log_chore_id = ChoreLog.chore_id
    AND Rating.week = ChoreLog.week
WHERE ChoreLog.week >= ?1
    -- when after_last_week is defined, ignore future ChoreLogs
    AND (?2 IS NULL OR ?2 > ChoreLog.week)
    AND ChoreLog.chore_id = ?3
GROUP BY ChoreLog.week, ChoreLog.chore_id, Tenant.name
ORDER BY ChoreLog.week;
"#,
            )
            .bind(start_week.unwrap_or(cur_week).db_week())
            // set after_last_week when a start date is set
            .bind(start_week.map(|_| cur_week.db_week()))
            .bind(chore.id)
            .fetch_all(&mut self.con)
            .await?;
            self.integrity_check().await?;
            let rows = chore_log_rows
                .into_iter()
                .map(|r| {
                    Ok(ChoreLogRow {
                        week: Week::from_db(r.try_get(0)?).to_string(),
                        tenant: r.try_get(1)?,
                        rating: r
                            .try_get::<Option<f32>, usize>(2)?
                            .map_or("".to_string(), |v| format!("{:.2}", v)),
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            out_mono += &format!(
                "{}## {}\nTimes performed: {}\n{}\n\n### Plan\n{}",
                if out_mono.is_empty() { "" } else { "\n\n\n" },
                chore.name,
                chore.times_performed,
                chore.description,
                Table::new(rows).modify(
                    Segment::all(),
                    Settings::new(Alignment::center(), Alignment::center())
                )
            );
        }

        Ok(ReplyMsg::from_mono(&format!("# Chores\n{}", out_mono)))
    }

    /// Go through all future ChoreLogs and figure out which ones to update. Update those.
    ///
    /// This function should be called every time something has changed that affects ChoreLogs
    /// planning (e.g. move in/out tenant, mark someone unwilling).
    pub async fn update_plan<F>(&mut self, fmt_replan_cmd: F) -> Result<ReplyMsg>
    where
        F: Fn(&str, Week) -> String,
    {
        self.clean_plan().await?;
        let mut out = ReplyMsg::new();
        let weeks_to_plan = self.get_weeks_to_plan().await?;
        for (week, chore) in &weeks_to_plan {
            out += self.plan_week(*week, chore, &fmt_replan_cmd).await?;
        }
        // only print full plan when something changed
        match weeks_to_plan.is_empty() {
            true => Ok(out),
            false => Ok(out + self.list_plan(None).await?),
        }
    }
}
