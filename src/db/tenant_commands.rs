use crate::db::*;

use anyhow::{bail, Result};
use sqlx::Row;
use tabled::{
    settings::{object::Segment, Alignment, Settings},
    Table, Tabled,
};

impl Db {
    /// Print a nice list of all rooms and their tenants.
    pub async fn list_tenants(&mut self) -> Result<ReplyMsg> {
        #[derive(Tabled)]
        struct RoomRow {
            room: String,
            tenant: String,
            #[tabled(rename = "score\neval")]
            score_rating: String,
        }
        let sql_rows = sqlx::query(
            r#"
SELECT Room.name, CONCAT(Tenant.name, CHAR(10), Tenant.chat_tag), TenantScoreSum.score, TenantRatingAvg.avg
FROM Room
-- Does a tenant live here currently?
LEFT JOIN LivesIn
    ON LivesIn.room_name = Room.name
    AND LivesIN.move_in_week <= ?1
    AND (
        LivesIn.move_out_week IS NULL
        OR LivesIn.move_out_week > ?1
    )
LEFT JOIN Tenant
    ON LivesIn.tenant_id = Tenant.id
LEFT JOIN TenantScoreSum ON TenantScoreSum.tenant_id = Tenant.id
-- get average rating for this tenant
LEFT JOIN (
    SELECT Tenant.id AS tenant_id, CAST(AVG(Rating.rating) AS FLOAT) AS avg
        FROM Tenant
        LEFT JOIN ChoreLog
            ON Tenant.id = ChoreLog.worker
        LEFT JOIN Rating
            ON Rating.for_chore_log_chore_id = ChoreLog.chore_id
            AND Rating.week = ChoreLog.week
        GROUP BY Tenant.id
    ) TenantRatingAvg ON TenantRatingAvg.tenant_id = Tenant.id
ORDER BY Room.name ASC;
"#,
        )
        .bind(self.week.db_week())
        .fetch_all(&mut self.con)
        .await?;
        self.integrity_check().await?;
        let rows = sql_rows
            .into_iter()
            .map(|r| {
                Ok(RoomRow {
                    room: r.try_get(0)?,
                    tenant: r.try_get::<Option<String>, usize>(1)?.unwrap_or_default(),
                    score_rating: format!(
                        "{}\n{}",
                        r.try_get::<Option<f32>, usize>(2)?
                            .map_or("".to_string(), |v| format!("{:.2}", v)),
                        r.try_get::<Option<f32>, usize>(3)?
                            .map_or("".to_string(), |v| format!("{:.2}", v)),
                    ),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(ReplyMsg::from_mono(&format!(
            "# Tenants\n\n{}",
            Table::new(rows).modify(
                Segment::all(),
                Settings::new(Alignment::center(), Alignment::center())
            )
        )))
    }

    /// Move someone into a room starting this week.
    pub async fn move_in<F>(
        &mut self,
        tenant: &str,
        tag: &Option<String>,
        room: &str,
        fmt_replan_cmd: F,
    ) -> Result<ReplyMsg>
    where
        F: Fn(&str, Week) -> String,
    {
        let tenant = Self::capitalize_tenant_name(tenant);
        if self.get_tenant_id(&tenant).await?.is_some() {
            if let Some(tag) = tag {
                let affected_rows = sqlx::query(
                    r#"
UPDATE Tenant
SET chat_tag = ?1
    WHERE Tenant.name = ?2;
"#,
                )
                .bind(tag)
                .bind(&tenant)
                .execute(&mut self.con)
                .await?
                .rows_affected();
                self.integrity_check().await?;
                if affected_rows != 1 {
                    bail!("affected {} rows", affected_rows);
                }
            }
        } else {
            // create new tenant
            let affected_rows = sqlx::query(
                r#"
INSERT INTO Tenant VALUES
    (NULL, ?1, ?2);
"#,
            )
            .bind(&tenant)
            .bind(tag)
            .execute(&mut self.con)
            .await?
            .rows_affected();
            self.integrity_check().await?;
            if affected_rows != 1 {
                bail!("affected {} rows", affected_rows);
            }
        }
        // assert the room is free
        if let Some(cur_tenant) = self.get_rooms_tenant(room).await? {
            return Ok(ReplyMsg::from_mono(&format!(
                "{} is living in room {}",
                cur_tenant, room
            )));
        }
        // assert the tenant isn't living anywhere else
        if let Some(cur_room) = self.get_tenants_room(&tenant).await? {
            return Ok(ReplyMsg::from_mono(&format!(
                "the tenant {} is currenlty living in {}, move them out of there first",
                tenant, cur_room
            )));
        }
        sqlx::query(
            r#"
INSERT INTO LivesIn VALUES
    (
        (SELECT Tenant.id FROM Tenant WHERE Tenant.name = ?1),
        ?2,
        ?3,
        NULL
    );
"#,
        )
        .bind(tenant)
        .bind(room)
        .bind(self.week.db_week())
        .execute(&mut self.con)
        .await?;
        self.integrity_check().await?;

        Ok(self.list_tenants().await? + self.update_plan(fmt_replan_cmd).await?)
    }

    /// Move someone out this week.
    pub async fn move_out<F>(&mut self, tenant: &str, fmt_replan_cmd: F) -> Result<ReplyMsg>
    where
        F: Fn(&str, Week) -> String,
    {
        let tenant = Self::capitalize_tenant_name(tenant);
        let room = match self.get_tenants_room(&tenant).await? {
            Some(r) => r,
            None => bail!("the tenant {} isn't living anywhere", tenant),
        };
        if !self.undo_move_in(&tenant, &room).await? {
            let affected_rows = sqlx::query(
                r#"
UPDATE LivesIn
SET move_out_week = ?1
    WHERE LivesIn.tenant_id = (SELECT Tenant.id FROM Tenant WHERE Tenant.name = ?2)
    AND LivesIn.move_in_week <= ?1
    AND (LivesIn.move_out_week IS NULL OR LivesIn.move_out_week > ?1);
"#,
            )
            .bind(self.week.db_week())
            .bind(tenant)
            .execute(&mut self.con)
            .await?
            .rows_affected();
            self.integrity_check().await?;
            if affected_rows != 1 {
                bail!("affected {} rows", affected_rows);
            }
        }
        Ok(self.list_tenants().await? + self.update_plan(fmt_replan_cmd).await?)
    }

    /// Create a new room.
    pub async fn create_room(&mut self, name: &str) -> Result<ReplyMsg> {
        let name = Self::capitalize_tenant_name(name);
        let affected_rows = sqlx::query(
            r#"
INSERT INTO Room VALUES
    (?1);
"#,
        )
        .bind(name)
        .execute(&mut self.con)
        .await?
        .rows_affected();
        self.integrity_check().await?;
        if affected_rows != 1 {
            bail!("affected {} rows", affected_rows);
        }
        self.list_tenants().await
    }
}
