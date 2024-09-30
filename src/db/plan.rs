use crate::db::*;

use anyhow::Result;
use rand::{distributions::Distribution, distributions::WeightedIndex};

impl Db {
    /// Remove all future ChoreLogs that aren't valid (anymore), i.e., because someone moved out.
    pub async fn clean_plan(&mut self) -> Result<()> {
        let sql_rows = sqlx::query(
            r#"
SELECT ChoreLog.week, Chore.name, Tenant.name
FROM ChoreLog
JOIN Chore ON Chore.id = ChoreLog.chore_id
JOIN Tenant ON Tenant.id = ChoreLog.worker
WHERE ChoreLog.week >= ?1;
"#,
        )
        .bind(self.get_week_internal().await.db_week())
        .fetch_all(&mut self.con)
        .await?;
        self.integrity_check().await?;

        let future_chore_logs = sql_rows
            .into_iter()
            .map(|r| -> Result<(Week, String, String)> {
                Ok((Week::from_db(r.try_get(0)?), r.try_get(1)?, r.try_get(2)?))
            })
            .collect::<Result<Vec<(Week, String, String)>>>()?;

        for (week, chore, tenant) in future_chore_logs {
            // Check if the chosen tenant is still available for that chore and that week.
            let available_tenants = self.get_available_tenants(week, &chore).await?;
            if !available_tenants
                .into_iter()
                .any(|(available_tenant, _)| available_tenant == tenant)
            {
                let affected_rows = sqlx::query(
                    r#"
DELETE FROM ChoreLog
WHERE ChoreLog.chore_id = (SELECT Chore.id FROM Chore WHERE Chore.name = ?1)
AND ChoreLog.week = ?2;
"#,
                )
                .bind(chore)
                .bind(week.db_week())
                .execute(&mut self.con)
                .await?
                .rows_affected();
                self.integrity_check().await?;
                if affected_rows != 1 {
                    bail!("affected {} rows", affected_rows);
                }
            }
        }
        Ok(())
    }

    /// Get all instances a ChoreLog should be created.
    ///
    /// Return list of (week, chore) tuples.
    pub async fn get_weeks_to_plan(&mut self) -> Result<Vec<(Week, String)>> {
        let mut weeks_to_plan = Vec::<(Week, String)>::new();
        for i in 0..self.weeks_to_plan {
            let check_week = Week::from_db(self.get_week_internal().await.db_week() + i as i64);
            let sql_rows = sqlx::query(
                r#"
SELECT Chore.name
FROM Chore
LEFT JOIN ChoreLog
    ON ChoreLog.chore_id = Chore.id
    AND ChoreLog.week = ?1
WHERE Chore.active = 1
AND ChoreLog.chore_id IS NULL;
"#,
            )
            .bind(check_week.db_week())
            .fetch_all(&mut self.con)
            .await?;
            self.integrity_check().await?;

            weeks_to_plan.append(
                &mut sql_rows
                    .into_iter()
                    .map(|r| -> Result<(Week, String)> { Ok((check_week, r.try_get(0)?)) })
                    .collect::<Result<Vec<(Week, String)>>>()?,
            );
        }
        Ok(weeks_to_plan)
    }

    /// Return list of (tenant, score) tuples in ascending order of score.
    ///
    /// The scores are the sum of all chores, not just the queried one.
    /// You need to normalize the scores so that they add up to 0.
    pub async fn get_available_tenants(
        &mut self,
        week: Week,
        chore: &str,
    ) -> Result<Vec<(String, f64)>> {
        let sql_rows = sqlx::query(
            r#"
-- This is very similar to the ProfitingTenant VIEW but excludes unwilling tenants, only consideres one week and includes the tenant's score.
-- The tenant's scores don't sum up to 0 as some tenants are excluded.
--
-- Only consider active chores.
SELECT Tenant.name, CAST(TenantScoreSum.score AS REAL)
FROM Tenant, Chore
JOIN TenantScoreSum
    ON Tenant.id = TenantScoreSum.tenant_id
-- ensure tenant lives here in the week
JOIN LivesIn
    ON Tenant.id = LivesIn.tenant_id
    AND LivesIn.move_in_week <= ?1
    AND (LivesIn.move_out_week IS NULL OR LivesIn.move_out_week > ?1)
-- ensure tenant doesn't have an exemption
LEFT JOIN TenantExemption
    ON TenantExemption.tenant_id = Tenant.id
    AND TenantExemption.start_week <= ?1
    AND (TenantExemption.end_week IS NULL OR TenantExemption.end_week > ?1)
-- ensure we only consider exemptions for the chore in question
LEFT JOIN ChoreExemption
    ON TenantExemption.exemption_reason_id = ChoreExemption.exemption_reason_id
    AND ChoreExemption.chore_id = Chore.id
-- ensure the tenant isn't unwilling for this week
LEFT JOIN Unwilling
    ON Unwilling.tenant_id = Tenant.id
    AND Unwilling.week = ?1
-- this must be on the LEFT JOIN with ChoreExemption as it is the second LEFT JOIN
WHERE ChoreExemption.chore_id IS NULL
AND Unwilling.tenant_id IS NULL
AND Chore.name = ?2
ORDER BY CAST(TenantScoreSum.score AS REAL) ASC;
"#,
        )
        .bind(week.db_week())
        .bind(chore)
        .fetch_all(&mut self.con)
        .await?;
        self.integrity_check().await?;

        let unnormalized_tenants = sql_rows
            .into_iter()
            .map(|r| -> Result<(String, f64)> { Ok((r.try_get(0)?, r.try_get(1)?)) })
            .collect::<Result<Vec<(String, f64)>>>()?;

        let sum = -unnormalized_tenants
            .iter()
            .fold(0.0, |sum, (_, s2)| sum + s2);
        let n = unnormalized_tenants.len() as f64;

        // Adjust all scores so that they sum up to 0.
        // This is done by taking the sum of score of all unavailable tenants and
        // distribute the score equally to all other tenants.
        let normalized_tenants: Vec<(String, f64)> = unnormalized_tenants
            .into_iter()
            .map(|(t, s)| (t, s + sum / n))
            .collect();

        let sum = -normalized_tenants.iter().fold(0.0, |sum, (_, s2)| sum + s2);
        debug_assert!(sum.abs() < 0.0001);

        Ok(normalized_tenants)
    }

    /// Convert every tenants score into a probability.
    ///
    /// The mathematical proof can be found in the repo.
    ///
    /// The tenants must be ordered in ascending order of score.
    ///
    /// return list of probabilities in same order as tenants
    pub fn calc_tenant_distribution(&self, tenants: Vec<(String, f64)>) -> Vec<f64> {
        let n = tenants.len() as f64;
        const EPS: f64 = 0.0000001;
        if tenants.is_empty() {
            return vec![];
        }
        // highest score
        let xn = tenants[tenants.len() - 1].1;
        if xn.abs() < EPS {
            return tenants.into_iter().map(|(_, _)| 1.0 / n).collect();
        }
        let dist: Vec<f64> = tenants
            .into_iter()
            .map(|(_, score)| {
                // floats are icky, don't return something slightly negative
                f64::max(
                    // the main affine-linear transformation
                    1.0 / n - (1.0 - self.gamma) / (n * xn) * score,
                    0.0,
                )
            })
            .collect();

        let sum: f64 = dist.iter().sum();
        debug_assert!(dist.iter().all(|p| *p > 0.0 || *p < 1.0));
        debug_assert!((sum - 1.0).abs() < 0.0001);

        dist
    }

    /// Choose a tenant from a list of tenants ordered by score.
    ///
    /// The list of tenants must not be empty.
    ///
    /// Return the tenants name, score and probability of being chosen.
    pub async fn choose_tenant(
        &mut self,
        tenants: Vec<(String, f64)>,
    ) -> Result<(String, f64, f64)> {
        let dist = self.calc_tenant_distribution(tenants.clone());
        println!("tenant Distribution: {:?}", dist);
        let idx = WeightedIndex::new(&dist)?.sample(&mut self.rng);
        Ok((tenants[idx].0.clone(), tenants[idx].1, dist[idx]))
    }

    /// Choose a tenant for a chore for a specific week and assign them the ChoreLog.
    /// Don't do anything when there are no available tenants.
    ///
    /// fmt_replan_cmd takes the tenant and week for the replan command
    pub async fn plan_week<F>(
        &mut self,
        week: Week,
        chore: &str,
        fmt_replan_cmd: F,
    ) -> Result<ReplyMsg>
    where
        F: FnOnce(&str, Week) -> String,
    {
        println!("planning {} {}", chore, week);
        let tenants = self.get_available_tenants(week, chore).await?;
        println!("available tenants: {:?}", tenants);
        if tenants.is_empty() {
            return Ok(ReplyMsg::new());
        }
        let (tenant, score, prob) = self.choose_tenant(tenants).await?;

        let row = sqlx::query(
            r#"
SELECT Tenant.chat_tag
FROM Tenant
WHERE Tenant.name = ?1;
"#,
        )
        .bind(&tenant)
        .fetch_one(&mut self.con)
        .await?;
        self.integrity_check().await?;
        let tag: Option<String> = row.try_get(0)?;

        let affected_rows = sqlx::query(
            r#"
INSERT INTO ChoreLog VALUES
(
    (SELECT Chore.id FROM Chore WHERE Chore.name = ?1),
    ?2,
    (SELECT Tenant.id FROM Tenant WHERE Tenant.name = ?3),
    0,
    NULL
)
"#,
        )
        .bind(chore)
        .bind(week.db_week())
        .bind(&tenant)
        .execute(&mut self.con)
        .await?
        .rows_affected();
        self.integrity_check().await?;
        if affected_rows != 1 {
            bail!("affected {} rows", affected_rows);
        }

        let week_delta = week.db_week() - self.get_week_internal().await.db_week();
        let mut msg = ReplyMsg::from_mono(&format!(
            "# {1} on {2} (in {6} {7}): {0}
{0}, you have been chosen for the {1} on {2}.
According to your effective score {3:.2} you've had a probability of {4:.0}% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    {5}
Alternatively you can move out and then back in if you're on vacation.",
            &tenant,
            chore,
            week,
            score,
            prob * 100.0,
            fmt_replan_cmd(&tenant, week),
            week_delta,
            if week_delta == 1 { "week" } else { "weeks" },
        ));
        if let Some(tag) = tag {
            msg.tags.insert(tag);
        }
        Ok(msg)
    }
}
