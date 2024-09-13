use crate::db::*;

use anyhow::Result;

impl Db {
    /// Create a new chore.
    pub async fn create_chore<F>(
        &mut self,
        name: &str,
        description: &str,
        fmt_replan_cmd: F,
    ) -> Result<ReplyMsg>
    where
        F: Fn(&str, Week) -> String,
    {
        let affected_rows = sqlx::query(
            r#"
INSERT INTO Chore VALUES
    (NULL, ?1, ?2, 1);
"#,
        )
        .bind(name)
        .bind(description)
        .execute(&mut self.con)
        .await?
        .rows_affected();
        self.integrity_check().await?;
        if affected_rows != 1 {
            bail!("affected {} rows", affected_rows);
        }

        let plan_update = self.update_plan(fmt_replan_cmd).await?;
        // only list plan separately when not already done in update
        match plan_update.mono_msg.is_empty() {
            true => Ok(self.list_plan(None).await? + plan_update),
            false => Ok(plan_update),
        }
    }

    /// Activate or Deactivate a chore.
    /// This substitutes deleting chores in the user interface.
    /// Deleting is complicated.
    pub async fn set_chore_active_state<F>(
        &mut self,
        name: &str,
        active_state: bool,
        fmt_replan_cmd: F,
    ) -> Result<ReplyMsg>
    where
        F: Fn(&str, Week) -> String,
    {
        let affected_rows = sqlx::query(
            r#"
UPDATE Chore
SET active = ?1
    WHERE Chore.name = ?2;
"#,
        )
        .bind(if active_state { 1 } else { 0 })
        .bind(name)
        .execute(&mut self.con)
        .await?
        .rows_affected();
        self.integrity_check().await?;
        if affected_rows != 1 {
            bail!("affected {} rows", affected_rows);
        }

        let plan_update = self.update_plan(fmt_replan_cmd).await?;
        // only list plan separately when not already done in update
        match plan_update.mono_msg.is_empty() {
            true => Ok(self.list_plan(None).await? + plan_update),
            false => Ok(plan_update),
        }
    }
}
