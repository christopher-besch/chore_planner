use crate::db::*;

use anyhow::Result;

impl Db {
    /// Get all the tenant living in a room or None when the room is vacant.
    pub async fn get_rooms_tenant(&mut self, room: &str) -> Result<Option<String>> {
        let rows = sqlx::query(
            r#"
SELECT Tenant.name
FROM Room
JOIN LivesIn
    ON Room.name = LivesIn.room_name
    AND LivesIn.move_in_week <= ?1
    AND (LivesIn.move_out_week IS NULL OR LivesIn.move_out_week > ?1)
JOIN Tenant ON LivesIn.tenant_id = Tenant.id
WHERE Room.name = ?2;
"#,
        )
        .bind(self.get_week_internal().await.db_week())
        .bind(room)
        .fetch_all(&mut self.con)
        .await?;
        self.integrity_check().await?;
        match rows.len() {
            0 => Ok(None),
            1 => Ok(Some(rows[0].try_get(0)?)),
            _ => bail!("get_rooms_tenant returned more than one row"),
        }
    }

    /// Get the room the tenant lives in or None when the tenant doesn't live here.
    pub async fn get_tenants_room(&mut self, name: &str) -> Result<Option<String>> {
        let rows = sqlx::query(
            r#"
SELECT Room.name
FROM Tenant
JOIN LivesIn
    ON Tenant.id = LivesIn.tenant_id
    AND LivesIn.move_in_week <= ?1
    AND (LivesIn.move_out_week IS NULL OR LivesIn.move_out_week > ?1)
JOIN Room ON LivesIn.room_name = Room.name
WHERE Tenant.name = ?2;
"#,
        )
        .bind(self.get_week_internal().await.db_week())
        .bind(name)
        .fetch_all(&mut self.con)
        .await?;
        self.integrity_check().await?;
        match rows.len() {
            0 => Ok(None),
            1 => Ok(Some(rows[0].try_get(0)?)),
            _ => bail!("get_tenants_room returned more than one row"),
        }
    }

    /// Get the id of a tenant.
    pub async fn get_tenant_id(&mut self, name: &str) -> Result<Option<u32>> {
        let rows = sqlx::query(
            r#"
SELECT Tenant.id
FROM Tenant
WHERE Tenant.name = ?1;
"#,
        )
        .bind(name)
        .fetch_all(&mut self.con)
        .await?;
        self.integrity_check().await?;
        match rows.len() {
            0 => Ok(None),
            1 => Ok(Some(rows[0].try_get(0)?)),
            _ => bail!("get_tenant_id returned more than one row"),
        }
    }

    /// When you move someone out the week they moved in, the LivesIn tuple needs to be deleted
    /// completely.
    /// This function does that and returns true iff a LivesIn tuple with this week's move in date
    /// existed and was deleted.
    pub async fn undo_move_in(&mut self, tenant: &str, room: &str) -> Result<bool> {
        let affected_rows = sqlx::query(
            r#"
DELETE
FROM LivesIn
WHERE LivesIn.tenant_id = (SELECT Tenant.id FROM Tenant WHERE Tenant.name = ?1)
AND LivesIn.room_name = ?2
And LivesIn.move_in_week = ?3;
"#,
        )
        .bind(tenant)
        .bind(room)
        .bind(self.get_week_internal().await.db_week())
        .execute(&mut self.con)
        .await?
        .rows_affected();
        self.integrity_check().await?;
        match affected_rows {
            0 => Ok(false),
            1 => Ok(true),
            _ => bail!("undo_move_in adjusted more than one row"),
        }
    }
}
