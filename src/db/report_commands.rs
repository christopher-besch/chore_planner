use crate::db::*;

use anyhow::Result;
use tabled::{
    settings::{object::Segment, Alignment, Settings},
    Table, Tabled,
};

impl Db {
    /// Print a nice report to see how things went (say last semester).
    pub async fn print_report(&mut self, week: Week) -> Result<ReplyMsg> {
        Ok(self.list_plan(Some(week)).await? + self.list_tenants().await?)
    }

    /// Print a nice report of who needs to do what job in the new week.
    pub async fn print_next_week_banner(&mut self) -> Result<ReplyMsg> {
        #[derive(Tabled)]
        struct JobRow {
            job: String,
            worker: String,
            #[tabled(skip)]
            tag: Option<String>,
        }
        let sql_rows = sqlx::query(
            r#"
SELECT Chore.name, Tenant.name, Tenant.chat_tag
FROM ChoreLog
JOIN Tenant ON ChoreLog.worker = Tenant.id
JOIN Chore ON ChoreLog.chore_id = Chore.id
WHERE ChoreLog.week = ?1;
"#,
        )
        .bind(self.week.db_week())
        .fetch_all(&mut self.con)
        .await?;
        self.integrity_check().await?;
        let rows = sql_rows
            .into_iter()
            .map(|r| {
                Ok(JobRow {
                    job: r.try_get(0)?,
                    worker: r.try_get(1)?,
                    tag: r.try_get(2)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let mut msg = ReplyMsg::from_mono(&format!(
            "# Week {}\nHello smart people!\nWe have another week and new jobs to go with it:\n\n{}\n\nHave a very safe and productive week.",
            self.week,
            Table::new(&rows).modify(
                Segment::all(),
                Settings::new(Alignment::center(), Alignment::center())
            )
        ));
        msg.tags = rows.into_iter().filter_map(|r| r.tag).collect();
        Ok(msg)
    }
}
