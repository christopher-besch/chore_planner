use crate::db::*;

use anyhow::Result;

impl Db {
    /// Is the tenant exempt by some reason?
    pub async fn is_tenant_exempt(&mut self, reason: &str, tenant: &str) -> Result<bool> {
        let rows = sqlx::query(
            r#"
SELECT *
FROM TenantExemption
    WHERE TenantExemption.tenant_id =
    (
        SELECT Tenant.id
        FROM Tenant
        WHERE Tenant.name = ?1
    )
    AND TenantExemption.exemption_reason_id =
    (
        SELECT ExemptionReason.id
        FROM ExemptionReason
        WHERE ExemptionReason.reason = ?2
    )
    AND TenantExemption.start_week <= ?3
    AND (TenantExemption.end_week IS NULL OR TenantExemption.end_week > ?3)
;
"#,
        )
        .bind(tenant)
        .bind(reason)
        .bind(self.get_week_internal().await.db_week())
        .fetch_all(&mut self.con)
        .await?;
        self.integrity_check().await?;
        match rows.len() {
            0 => Ok(false),
            1 => Ok(true),
            _ => bail!("is_tenant_exempt returned more than one row"),
        }
    }
}
