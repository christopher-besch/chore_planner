use crate::db::*;

use anyhow::Result;
use tabled::{
    settings::{object::Segment, Alignment, Settings},
    Table, Tabled,
};

impl Db {
    /// Print a nice list of all exemptions.
    pub async fn list_exemptions(&mut self) -> Result<ReplyMsg> {
        #[derive(Tabled)]
        struct ExemptionRow {
            reason: String,
            chores: String,
            tenants: String,
        }
        let sql_rows = sqlx::query(
            r#"
SELECT ExemptionReason.reason, Chores.chores, Tenants.tenants
FROM ExemptionReason
LEFT JOIN (
    SELECT ChoreExemption.exemption_reason_id AS exemption_reason_id, GROUP_CONCAT(Chore.name, Char(10)) AS chores
    FROM ChoreExemption
    JOIN Chore
        ON ChoreExemption.chore_id = Chore.id
    WHERE Chore.active = 1
    GROUP BY ChoreExemption.exemption_reason_id
) Chores ON Chores.exemption_reason_id = ExemptionReason.id
LEFT JOIN (
    SELECT TenantExemption.exemption_reason_id AS exemption_reason_id, GROUP_CONCAT(Tenant.name, Char(10)) AS tenants
    FROM TenantExemption
    JOIN Tenant
        ON Tenant.id = TenantExemption.tenant_id
    WHERE TenantExemption.start_week <= ?1
    AND (TenantExemption.end_week IS NULL OR TenantExemption.end_week > ?1)
    GROUP BY TenantExemption.exemption_reason_id
) Tenants ON Tenants.exemption_reason_id = ExemptionReason.id;
"#,
        )
        .bind(self.get_week_internal().await.db_week())
        .fetch_all(&mut self.con)
        .await?;
        self.integrity_check().await?;
        let rows = sql_rows
            .into_iter()
            .map(|r| {
                Ok(ExemptionRow {
                    reason: r.try_get(0)?,
                    chores: r.try_get(1)?,
                    tenants: r.try_get(2)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(ReplyMsg::from_mono(&format!(
            "# Exemptions\n\n{}",
            Table::new(rows).modify(
                Segment::all(),
                Settings::new(Alignment::center(), Alignment::center())
            )
        )))
    }

    /// Create a new exemption reason without granting it to anyone.
    /// This also sets the chores the exemption is for.
    pub async fn create_exemption_reason(
        &mut self,
        reason: &str,
        chores: &Vec<String>,
    ) -> Result<ReplyMsg> {
        // create ExemptionReason
        let res = sqlx::query(
            r#"
INSERT INTO ExemptionReason VALUES
    (NULL, ?1);
"#,
        )
        .bind(reason)
        .execute(&mut self.con)
        .await?;
        self.integrity_check().await?;
        let reason_id = res.last_insert_rowid();
        if res.rows_affected() != 1 {
            bail!("affected {} ExemptionReason rows", res.rows_affected());
        }

        // apply Chores to ExemptionReason
        for chore in chores {
            let affected_rows = sqlx::query(
                r#"
INSERT INTO ChoreExemption VALUES
    ((SELECT Chore.id FROM Chore WHERE Chore.name = ?1), ?2);
"#,
            )
            .bind(chore)
            .bind(reason_id)
            .execute(&mut self.con)
            .await?
            .rows_affected();
            self.integrity_check().await?;
            if affected_rows != 1 {
                bail!("affected {} ChoreExemption rows", affected_rows);
            }
        }
        self.list_exemptions().await
    }

    /// Update the set of chores an exemption reason is for.
    pub async fn change_exemption_reason<F>(
        &mut self,
        reason: &str,
        chores: &Vec<String>,
        fmt_replan_cmd: F,
    ) -> Result<ReplyMsg>
    where
        F: Fn(&str, Week) -> String,
    {
        if sqlx::query(
            r#"
SELECT *
FROM ExemptionReason
WHERE ExemptionReason.reason = ?1;
"#,
        )
        .bind(reason)
        .fetch_all(&mut self.con)
        .await?
        .is_empty()
        {
            bail!("the ExemptionReason {} doesn't exist", reason);
        }

        // delete all current ChoreExemptions
        sqlx::query(
            r#"
DELETE FROM ChoreExemption
WHERE ChoreExemption.exemption_reason_id = (
    SELECT ExemptionReason.id
    FROM ExemptionReason
    WHERE ExemptionReason.reason = ?1
);
"#,
        )
        .bind(reason)
        .execute(&mut self.con)
        .await?;
        self.integrity_check().await?;
        // set new ChoreExemptions
        for chore in chores {
            let affected_rows = sqlx::query(
                r#"
INSERT INTO ChoreExemption VALUES
((
    SELECT Chore.id
    FROM Chore
    WHERE Chore.name = ?1
),(
    SELECT ExemptionReason.id
    FROM ExemptionReason
    WHERE ExemptionReason.reason = ?2
));
"#,
            )
            .bind(chore)
            .bind(reason)
            .execute(&mut self.con)
            .await?
            .rows_affected();
            self.integrity_check().await?;
            if affected_rows != 1 {
                bail!("affected {} ChoreExemption rows", affected_rows);
            }
        }
        Ok(self.list_exemptions().await? + self.update_plan(fmt_replan_cmd).await?)
    }

    /// Grant an exemption reason to a tenant.
    pub async fn grant_exemption<F>(
        &mut self,
        reason: &str,
        tenant: &str,
        fmt_replan_cmd: F,
    ) -> Result<ReplyMsg>
    where
        F: Fn(&str, Week) -> String,
    {
        let tenant = Self::capitalize_tenant_name(tenant);
        if self.is_tenant_exempt(reason, &tenant).await? {
            bail!("tenant is already exempt");
        }
        let affected_rows = sqlx::query(
            r#"
INSERT INTO TenantExemption VALUES
((
    SELECT Tenant.id
    FROM Tenant
    WHERE Tenant.name = ?1
),(
    SELECT ExemptionReason.id
    FROM ExemptionReason
    WHERE ExemptionReason.reason = ?2
), ?3, NULL);
"#,
        )
        .bind(tenant)
        .bind(reason)
        .bind(self.get_week_internal().await.db_week())
        .execute(&mut self.con)
        .await?
        .rows_affected();
        self.integrity_check().await?;
        if affected_rows != 1 {
            bail!("affected {} TenantExemption rows", affected_rows);
        }

        Ok(self.list_exemptions().await? + self.update_plan(fmt_replan_cmd).await?)
    }

    /// Undo granting an exemption to a tenant.
    pub async fn revoke_exemption<F>(
        &mut self,
        reason: &str,
        tenant: &str,
        fmt_replan_cmd: F,
    ) -> Result<ReplyMsg>
    where
        F: Fn(&str, Week) -> String,
    {
        let tenant = Self::capitalize_tenant_name(tenant);
        if !self.is_tenant_exempt(reason, &tenant).await? {
            bail!("tenant is not exempt");
        }
        let affected_rows = sqlx::query(
            r#"
UPDATE TenantExemption
SET end_week = ?1
    WHERE TenantExemption.tenant_id =
    (
        SELECT Tenant.id
        FROM Tenant
        WHERE Tenant.name = ?2
    )
    AND TenantExemption.exemption_reason_id =
    (
        SELECT ExemptionReason.id
        FROM ExemptionReason
        WHERE ExemptionReason.reason = ?3
    )
    AND TenantExemption.start_week <= ?1
    AND (TenantExemption.end_week IS NULL OR TenantExemption.end_week > ?1);
"#,
        )
        .bind(self.get_week_internal().await.db_week())
        .bind(tenant)
        .bind(reason)
        .execute(&mut self.con)
        .await?
        .rows_affected();
        self.integrity_check().await?;
        if affected_rows != 1 {
            bail!("affected {} TenantExemption rows", affected_rows);
        }
        Ok(self.list_exemptions().await? + self.update_plan(fmt_replan_cmd).await?)
    }
}
