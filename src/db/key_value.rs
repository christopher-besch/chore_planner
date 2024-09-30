use crate::db::*;

impl Db {
    pub async fn set_week_internal(&mut self, week: Week) {
        self.set_kv("current_week", &week.db_week().to_string())
            .await
    }

    pub async fn get_week_internal(&mut self) -> Week {
        match self.get_kv("current_week").await {
            Some(s) => Week::from_db(s.parse::<i64>().unwrap()),
            None => self.fallback_week,
        }
    }

    /// get the value of a key from the database
    /// panics on error
    async fn get_kv(&mut self, key: &str) -> Option<String> {
        let rows = sqlx::query(
            r#"
SELECT KeyValue.value
FROM KeyValue
WHERE KeyValue.key = ?1;
"#,
        )
        .bind(key)
        .fetch_all(&mut self.con)
        .await
        .unwrap();
        self.integrity_check().await.unwrap();
        match rows.len() {
            0 => None,
            1 => Some(rows[0].try_get(0).unwrap()),
            _ => panic!("get_key returned more than one row"),
        }
    }

    /// set the value of a key from the database
    /// panics on error
    async fn set_kv(&mut self, key: &str, value: &str) {
        let affected_rows = sqlx::query(
            r#"
REPLACE INTO KeyValue VALUES
    (?1, ?2);
"#,
        )
        .bind(key)
        .bind(value)
        .execute(&mut self.con)
        .await
        .unwrap()
        .rows_affected();
        self.integrity_check().await.unwrap();
        if affected_rows != 1 {
            panic!("affected {} rows", affected_rows);
        }
    }
}
