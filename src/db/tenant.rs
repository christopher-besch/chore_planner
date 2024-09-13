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
        .bind(self.week.db_week())
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
        .bind(self.week.db_week())
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
}
