use crate::db::*;

use anyhow::{bail, Context, Result};
use sqlx::Row;

impl Db {
    /// Migrate the database from an old version to the current scheme.
    pub async fn migrate(&mut self) -> Result<()> {
        let migrations = vec![
            r#"
CREATE TABLE Room (
    name TEXT NOT NULL,
    --
    CONSTRAINT Room_PK PRIMARY KEY (name)
) STRICT;
"#,
            r#"
CREATE TABLE Tenant (
    id INTEGER PRIMARY KEY,
    -- must be in Titel Case
    name TEXT NOT NULL,
    -- can be NULL e.g., when the user doesn't have a telegram username
    chat_tag TEXT,
    --
    UNIQUE (name)
) STRICT;
"#,
            r#"
CREATE TABLE LivesIn (
    tenant_id INTEGER NOT NULL,
    room_name TEXT NOT NULL,
    -- available for chores from this week on
    -- weeks are stored as weeks from the first week of 1970
    move_in_week INTEGER NOT NULL,
    -- unavailable for chores from this week on
    move_out_week INTEGER,
    --
    CONSTRAINT LivesIn_PK PRIMARY KEY (room_name, move_in_week),
    CONSTRAINT LivesIn_TO_Tenant_FK FOREIGN KEY (tenant_id) REFERENCES Tenant (id),
    CONSTRAINT LivesIn_TO_Room_FK FOREIGN KEY (room_name) REFERENCES Room (name)
) STRICT;
"#,
            r#"
CREATE TABLE Unwilling (
    tenant_id INTEGER NOT NULL,
    week INTEGER NOT NULL,
    --
    CONSTRAINT Unwilling_PK PRIMARY KEY (tenant_id, week),
    CONSTRAINT Unwilling_TO_Tenant_FK FOREIGN KEY (tenant_id) REFERENCES Tenant (id)
) STRICT;
"#,
            r#"
CREATE TABLE Chore (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    active INTEGER NOT NULL,
    --
    UNIQUE(name)
) STRICT;
"#,
            r#"
CREATE TABLE ExemptionReason (
    id INTEGER PRIMARY KEY,
    reason TEXT NOT NULL,
    --
    UNIQUE (reason)
) STRICT;
"#,
            r#"
CREATE TABLE TenantExemption (
    tenant_id INTEGER NOT NULL,
    exemption_reason_id INTEGER NOT NULL,
    start_week INTEGER NOT NULL,
    end_week INTEGER,
    --
    CONSTRAINT TenantExemption_PK PRIMARY KEY (tenant_id, exemption_reason_id, start_week),
    CONSTRAINT TenantExemption_TO_Tenant_FK FOREIGN KEY (tenant_id) REFERENCES Tenant (id),
    CONSTRAINT TenantExemption_TO_ExemptionReason_FK FOREIGN KEY (exemption_reason_id) REFERENCES ExemptionReason (id)
) STRICT;
"#,
            r#"
CREATE TABLE ChoreExemption (
    chore_id INTEGER NOT NULL,
    exemption_reason_id INTEGER NOT NULL,
    --
    CONSTRAINT ChoreExemption_PK PRIMARY KEY (chore_id, exemption_reason_id),
    CONSTRAINT ChoreExemption_TO_Tenant_FK FOREIGN KEY (chore_id) REFERENCES Chore (id),
    CONSTRAINT ChoreExemption_TO_ExemptionReason_FK FOREIGN KEY (exemption_reason_id) REFERENCES ExemptionReason (id)
) STRICT;
"#,
            r#"
CREATE TABLE ChoreLog (
    chore_id INTEGER NOT NULL,
    week INTEGER NOT NULL,
    worker INTEGER NOT NULL,
    -- the week with this CoreLog has been processed (i.e., a rating request (i.e., a telegram poll) has been issued and stopped)
    -- this usually happens a week after the actual chore was to be done
    -- this may only be set to 1 when a rating_poll_id it has been set
    completed INTEGER NOT NULL,
    -- when the rating poll is stopped, we need to find the ChoreLog it belongs to
    rating_poll_id INTEGER,
    --
    CONSTRAINT ChoreLog_PK PRIMARY KEY (chore_id, week),
    CONSTRAINT ChoreLog_TO_Tenant_FK FOREIGN KEY (worker) REFERENCES Tenant (id),
    UNIQUE (rating_poll_id)
) STRICT;
"#,
            r#"
CREATE TABLE Rating (
    id INTEGER PRIMARY KEY,
    for_chore_log_chore_id INTEGER NOT NULL,
    week INTEGER NOT NULL,
    rating INTEGER NOT NULL,
    --
    CONSTRAINT Rating_TO_planned_ChoreLog_FK FOREIGN KEY (for_chore_log_chore_id, week) REFERENCES ChoreLog (chore_id, week)
) STRICT;
"#,
            r#"
-- List all tenants that should do a chore at some week (i.e., that are profiting from a certain chore at some week without an exemption).
--
-- tenant_id the tenant that profited
-- chore_id the chore the tenant profited from
-- week the week the tenant profited from the chore
-- did_work 1 iff the tenant did the chore themselfes, else 0
CREATE VIEW ProfitingTenant (tenant_id, chore_id, week, did_work) AS
SELECT Tenant.id, ChoreLog.chore_id, ChoreLog.week, IIF(ChoreLog.worker = Tenant.id, 1, 0)
FROM Tenant
-- ensure tenant lives here in the week
JOIN LivesIn ON Tenant.id = LivesIn.tenant_id
JOIN ChoreLog ON
    LivesIn.move_in_week <= ChoreLog.week
    AND (LivesIn.move_out_week IS NULL OR LivesIn.move_out_week > ChoreLog.week)
-- ensure tenant doesn't have an exemption
LEFT JOIN TenantExemption
    ON TenantExemption.tenant_id = Tenant.id
    AND TenantExemption.start_week <= ChoreLog.week
    AND (TenantExemption.end_week IS NULL OR TenantExemption.end_week > ChoreLog.week)
-- ensure we only consider exemptions for the chore in question
LEFT JOIN ChoreExemption
    ON TenantExemption.exemption_reason_id = ChoreExemption.exemption_reason_id
    AND ChoreExemption.chore_id = ChoreLog.chore_id
-- this must be on the LEFT JOIN with ChoreExemption as it is the second LEFT JOIN
WHERE ChoreExemption.chore_id IS NULL;
"#,
            r#"
-- chore_id the chore
-- week the week
-- count the amount of tenants profiting from the chore being done in that week
CREATE VIEW TotalProfitingTenant (chore_id, week, count) AS
SELECT ProfitingTenant.chore_id, ProfitingTenant.week, COUNT(DISTINCT ProfitingTenant.tenant_id)
FROM ProfitingTenant
GROUP BY ProfitingTenant.week, ProfitingTenant.chore_id;
"#,
            r#"
-- The score for each tenant represents how often they did chores and how often they profited from others doing chores.
-- A score of 0 means that the tenant profited from other people's chores just as much as they cumulatively did from her.
-- This is the fair equilibrium the planning algorithm is converging against.
-- A negative score means they profited more from the other chores than they should've.
-- A positiv score means they worked on the chore on more weeks than they needed to.
-- A tenant exempted for a chore is not counted as profiting from that chore.
--
-- Look at every week there is a ChoreLog and all N tenants profiting from that chore.
-- When the tenant did the work they are aworded 1 point; everyone else get's 1/N points deducted.
-- This is value is added up over all weeks creating the score.
--
-- tenant_id the tenant in question
-- chore_id the chore in question
-- score the score of the tenant for that score
CREATE VIEW TenantScore (tenant_id, chore_id, score) AS
SELECT Tenant.id, Chore.id,
    COALESCE(SUM(ProfitingTenant.did_work * 1 - (1-ProfitingTenant.did_work) * 1/CAST(TotalProfitingTenant.count - 1 AS FLOAT)), 0)
FROM Tenant, Chore
LEFT JOIN ProfitingTenant ON ProfitingTenant.tenant_id = Tenant.id AND ProfitingTenant.chore_id = Chore.id
LEFT JOIN TotalProfitingTenant ON ProfitingTenant.chore_id = TotalProfitingTenant.chore_id
    AND ProfitingTenant.week = TotalProfitingTenant.week
GROUP BY Tenant.id, Chore.id
ORDER BY Chore.id, Tenant.id;
"#,
        ];

        let mut next_migration = self.get_user_version().await?;
        println!("next_migration: {}", next_migration);
        while let Some(migration) = migrations.get(next_migration as usize) {
            println!("applying migration {}", next_migration);
            sqlx::query(migration).execute(&mut self.con).await?;
            next_migration += 1;
            self.set_user_version(next_migration).await?;
        }
        println!("migration done");
        Ok(())
    }

    /// Get the database's user_version.
    /// The user_version is used to store what migration to apply next.
    async fn get_user_version(&mut self) -> Result<u32> {
        let row = sqlx::query(r#"PRAGMA user_version;"#)
            .fetch_one(&mut self.con)
            .await?;
        row.try_get::<u32, usize>(0)
            .context("get_user_version failed")
    }

    /// Set the database's user_version.
    /// The user_version is used to store what migration to apply next.
    async fn set_user_version(&mut self, user_version: u32) -> Result<()> {
        sqlx::query(&format!("PRAGMA user_version = {};", user_version))
            .execute(&mut self.con)
            .await?;
        assert_eq!(self.get_user_version().await?, user_version);
        Ok(())
    }

    /// Perform all integrity checks the database can't automatically ensure itself.
    pub async fn integrity_check(&mut self) -> Result<()> {
        // sql integrity_check
        let row = sqlx::query(r#"PRAGMA integrity_check;"#)
            .fetch_one(&mut self.con)
            .await?;
        assert_eq!(row.try_get::<String, usize>(0)?, "ok");

        // local integrity conditions SQL doesn't cover
        if !sqlx::query(
            r#"
SELECT *
FROM LivesIn
WHERE LivesIn.move_out_week IS NOT NULL AND LivesIn.move_out_week < LivesIn.move_in_week;
"#,
        )
        .fetch_all(&mut self.con)
        .await?
        .is_empty()
        {
            bail!("move out date is before move in date");
        }

        if !sqlx::query(
            r#"
SELECT *
FROM TenantExemption
WHERE TenantExemption.end_week IS NOT NULL AND TenantExemption.end_week < TenantExemption.start_week;
"#,
        )
        .fetch_all(&mut self.con)
        .await?
        .is_empty()
        {
            bail!("exemption end date is before start date");
        }

        let rows = sqlx::query(
            r#"
SELECT Tenant.name
FROM Tenant;
"#,
        )
        .fetch_all(&mut self.con)
        .await?;
        for row in rows {
            let name = row.try_get(0)?;
            if name != Self::capitalize_tenant_name(name) {
                bail!("Tenant name {} is not capitalized", name);
            }
        }

        // global integrity conditions SQL doesn't cover
        if !sqlx::query(
            r#"
SELECT *
FROM Room
JOIN LivesIn LivesInA
    ON Room.name = LivesInA.room_name
JOIN LivesIn LivesInB
    ON Room.name = LivesInB.room_name
    AND LivesInA.tenant_id != LivesInB.tenant_id
    -- B moves in before A moves out
    AND (LivesInA.move_out_week IS NULL OR LivesInA.move_out_week > LivesInB.move_in_week)
    -- B moves out after A moves in
    AND (LivesInB.move_out_week IS NULL OR LivesInA.move_in_week < LivesInB.move_out_week);
"#,
        )
        .fetch_all(&mut self.con)
        .await?
        .is_empty()
        {
            bail!("there is a room with multiple tenants at the same time");
        }
        if !sqlx::query(
            r#"
SELECT *
FROM Room
JOIN LivesIn LivesInA
    ON Room.name = LivesInA.room_name
JOIN LivesIn LivesInB
    ON Room.name != LivesInB.room_name
    AND LivesInA.tenant_id = LivesInB.tenant_id
    -- B moves in before A moves out
    AND (LivesInA.move_out_week IS NULL OR LivesInA.move_out_week > LivesInB.move_in_week)
    -- B moves out after A moves in
    AND (LivesInB.move_out_week IS NULL OR LivesInA.move_in_week < LivesInB.move_out_week);
"#,
        )
        .fetch_all(&mut self.con)
        .await?
        .is_empty()
        {
            bail!("there is a tenant with multiple rooms at the same time");
        }
        if !sqlx::query(
            r#"
SELECT *
FROM TenantExemption TenantExemptionA, TenantExemption TenantExemptionB
    WHERE TenantExemptionA.tenant_id = TenantExemptionB.tenant_id
    AND TenantExemptionA.exemption_reason_id = TenantExemptionB.exemption_reason_id
    AND TenantExemptionA.start_week != TenantExemptionB.start_week
    -- B starts before A ends
    AND (TenantExemptionA.end_week IS NULL OR TenantExemptionA.end_week > TenantExemptionB.start_week)
    -- B ends after A starts
    AND (TenantExemptionB.end_week IS NULL OR TenantExemptionA.start_week < TenantExemptionB.end_week);
"#,
        )
        .fetch_all(&mut self.con)
        .await?
        .is_empty()
        {
            bail!("there are two overlapping TenantExemptions for the same Tenant and ExemptionReason");
        }
        if !sqlx::query(
            r#"
SELECT *
FROM ChoreLog
    WHERE ChoreLog.completed = 1
    AND ChoreLog.rating_poll_id IS NULL;
"#,
        )
        .fetch_all(&mut self.con)
        .await?
        .is_empty()
        {
            bail!("there is a completed ChoreLog without a poll id");
        }
        Ok(())
    }
}
