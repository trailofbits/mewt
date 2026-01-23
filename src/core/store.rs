use chrono::{DateTime, Utc};
use sqlx::sqlite::SqlitePool;
use std::path::PathBuf;

use crate::types::{
    CampaignSummary, Hash, Mutant, Outcome, Status, StoreError, StoreResult, Target,
};

#[derive(Clone, Debug)]
pub struct SqlStore {
    pool: SqlitePool,
}

impl SqlStore {
    pub async fn new(sqlite_connection_string: String) -> StoreResult<Self> {
        let pool = SqlitePool::connect(&sqlite_connection_string).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn add_target(&self, target: Target) -> StoreResult<i64> {
        // Get language string
        let language_str = &target.language;

        let file_hash_hex = target.file_hash.to_hex();
        let path_str = target.path.to_string_lossy().into_owned();
        let existing = sqlx::query!(
            r#"
            SELECT id, path
            FROM targets
            WHERE file_hash = ?
        "#,
            file_hash_hex
        )
        .fetch_optional(&self.pool)
        .await?;
        match existing {
            // got an exact match
            Some(record) if record.path == path_str => Ok(record.id),
            // file was moved, update path
            Some(record) => {
                sqlx::query!(
                    r#"
                    UPDATE targets
                    SET path = ?
                    WHERE id = ?
                "#,
                    path_str,
                    record.id
                )
                .execute(&self.pool)
                .await?;
                Ok(record.id)
            }
            // this target doesn't exist yet, insert it
            None => {
                let result = sqlx::query!(
                    r#"
                    INSERT INTO targets (path, file_hash, text, language)
                    VALUES (?, ?, ?, ?)
                "#,
                    path_str,
                    file_hash_hex,
                    target.text,
                    language_str
                )
                .execute(&self.pool)
                .await?;
                Ok(result.last_insert_rowid())
            }
        }
    }

    // returns None if noop bc mutant already exists
    // otherwise returns the newly added mutant id
    pub async fn add_mutant(&self, mutant: Mutant) -> StoreResult<Option<i64>> {
        let existing = sqlx::query!(
            r#"
            SELECT id
            FROM mutants
            WHERE target_id = ? AND byte_offset = ? AND old_text = ? AND new_text = ? AND mutation_slug = ?
        "#,
            mutant.target_id,
            mutant.byte_offset,
            mutant.old_text,
            mutant.new_text,
            mutant.mutation_slug,
        )
        .fetch_optional(&self.pool)
        .await?;
        match existing {
            Some(_) => Ok(None),
            None => {
                let result = sqlx::query!(
                    r#"
                INSERT INTO mutants (target_id, byte_offset, line_offset, old_text, new_text, mutation_slug)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
                    mutant.target_id,
                    mutant.byte_offset,
                    mutant.line_offset,
                    mutant.old_text,
                    mutant.new_text,
                    mutant.mutation_slug,
                )
                .execute(&self.pool)
                .await?;
                Ok(Some(result.last_insert_rowid()))
            }
        }
    }

    pub async fn add_outcome(&self, outcome: Outcome) -> StoreResult<i64> {
        let status_str = outcome.status.to_string();
        let time_str = outcome.time.to_rfc3339();
        let existing = sqlx::query!(
            r#"
            SELECT mutant_id
            FROM outcomes
            WHERE mutant_id = ?
        "#,
            outcome.mutant_id
        )
        .fetch_optional(&self.pool)
        .await?;
        match existing {
            // Update existing outcome
            Some(_) => {
                sqlx::query!(
                    r#"
                    UPDATE outcomes
                    SET status = ?, output = ?, time = ?, duration_ms = ?
                    WHERE mutant_id = ?
                "#,
                    status_str,
                    outcome.output,
                    time_str,
                    outcome.duration_ms,
                    outcome.mutant_id
                )
                .execute(&self.pool)
                .await?;
                Ok(outcome.mutant_id)
            }
            // Insert new outcome
            None => {
                sqlx::query!(
                    r#"
                    INSERT INTO outcomes (mutant_id, status, output, time, duration_ms)
                    VALUES (?, ?, ?, ?, ?)
                "#,
                    outcome.mutant_id,
                    status_str,
                    outcome.output,
                    time_str,
                    outcome.duration_ms,
                )
                .execute(&self.pool)
                .await?;
                Ok(outcome.mutant_id)
            }
        }
    }

    pub async fn get_target(&self, target_id: i64) -> StoreResult<Target> {
        let record = sqlx::query!(
            r#"
            SELECT id, path, file_hash, text, language
            FROM targets
            WHERE id = ?
        "#,
            target_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StoreError::NotFound(target_id),
            e => StoreError::DatabaseError(e),
        })?;
        let language = record.language;

        Ok(Target {
            id: record.id,
            path: PathBuf::from(record.path),
            file_hash: Hash::try_from(record.file_hash)?,
            text: record.text,
            language,
        })
    }

    pub async fn get_all_targets(&self) -> StoreResult<Vec<Target>> {
        let records = sqlx::query!(
            r#"
            SELECT id, path, file_hash, text, language
            FROM targets
        "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut targets = Vec::with_capacity(records.len());
        for record in records {
            targets.push(Target {
                id: record.id,
                path: PathBuf::from(record.path),
                file_hash: Hash::try_from(record.file_hash)?,
                text: record.text,
                language: record.language,
            });
        }

        Ok(targets)
    }

    pub async fn get_mutant(&self, id: i64) -> StoreResult<Mutant> {
        let result = sqlx::query!(
            r#"
            SELECT id, target_id, byte_offset, line_offset, old_text, new_text, mutation_slug
            FROM mutants
            WHERE id = ?
        "#,
            id
        )
        .fetch_optional(&self.pool)
        .await;
        match result {
            Ok(Some(r)) => Ok(Mutant {
                id: r.id,
                target_id: r.target_id,
                byte_offset: r.byte_offset as u32,
                line_offset: r.line_offset as u32,
                old_text: r.old_text,
                new_text: r.new_text,
                mutation_slug: r.mutation_slug,
            }),
            Ok(None) => Err(StoreError::NotFound(id)),
            Err(e) => Err(StoreError::DatabaseError(e)),
        }
    }

    pub async fn get_mutants(&self, target_id: i64) -> StoreResult<Vec<Mutant>> {
        let records = sqlx::query!(
            r#"
            SELECT id, target_id, byte_offset, line_offset, old_text, new_text, mutation_slug
            FROM mutants
            WHERE target_id = ?
        "#,
            target_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(records
            .into_iter()
            .map(|r| Mutant {
                id: r.id,
                target_id: r.target_id,
                byte_offset: r.byte_offset as u32,
                line_offset: r.line_offset as u32,
                old_text: r.old_text,
                new_text: r.new_text,
                mutation_slug: r.mutation_slug,
            })
            .collect())
    }

    pub async fn get_outcome(&self, mutant_id: i64) -> StoreResult<Option<Outcome>> {
        let record = sqlx::query!(
            r#"
            SELECT mutant_id, status, output, time AS "time: String", duration_ms
            FROM outcomes
            WHERE mutant_id = ?
        "#,
            mutant_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(match record {
            Some(r) => Some(Outcome {
                mutant_id: r.mutant_id,
                status: r
                    .status
                    .parse::<Status>()
                    .map_err(|e| StoreError::InvalidStatus(e.to_string()))?,
                output: r.output,
                time: DateTime::parse_from_rfc3339(&r.time).map(|dt| dt.with_timezone(&Utc))?,
                duration_ms: r.duration_ms as u32,
            }),
            None => None,
        })
    }

    pub async fn remove_target(&self, target_id: i64) -> StoreResult<()> {
        sqlx::query!(
            r#"
            DELETE FROM targets
            WHERE id = ?
        "#,
            target_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_outcomes(&self, target_id: i64) -> StoreResult<Vec<Outcome>> {
        let records = sqlx::query!(
            r#"
            SELECT o.mutant_id, o.status, o.output, o.time AS "time: String", o.duration_ms
            FROM outcomes o
            JOIN mutants m ON o.mutant_id = m.id
            WHERE m.target_id = ?
            "#,
            target_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut outcomes = Vec::with_capacity(records.len());
        for r in records {
            outcomes.push(Outcome {
                mutant_id: r.mutant_id,
                status: r
                    .status
                    .parse::<Status>()
                    .map_err(|e| StoreError::InvalidStatus(e.to_string()))?,
                output: r.output,
                time: DateTime::parse_from_rfc3339(&r.time).map(|dt| dt.with_timezone(&Utc))?,
                duration_ms: r.duration_ms as u32,
            });
        }

        Ok(outcomes)
    }

    pub async fn get_mutants_without_outcomes(&self) -> StoreResult<Vec<Mutant>> {
        let records = sqlx::query!(
            r#"
            SELECT m.id, m.target_id, m.byte_offset, m.line_offset, m.old_text, m.new_text, m.mutation_slug
            FROM mutants m
            LEFT JOIN outcomes o ON m.id = o.mutant_id
            WHERE o.mutant_id IS NULL
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records
            .into_iter()
            .map(|r| Mutant {
                id: r.id,
                target_id: r.target_id,
                byte_offset: r.byte_offset as u32,
                line_offset: r.line_offset as u32,
                old_text: r.old_text,
                new_text: r.new_text,
                mutation_slug: r.mutation_slug,
            })
            .collect())
    }

    pub async fn get_mutants_to_test(&self) -> StoreResult<(Vec<Mutant>, usize, usize)> {
        // First get mutants without any outcomes
        let untested_mutants = self.get_mutants_without_outcomes().await?;
        let untested_count = untested_mutants.len();

        // Then get mutants with Timeout status (to be retested)
        let timeout_records = sqlx::query!(
            r#"
            SELECT m.id, m.target_id, m.byte_offset, m.line_offset, m.old_text, m.new_text, m.mutation_slug
            FROM mutants m
            JOIN outcomes o ON m.id = o.mutant_id
            WHERE o.status = 'Timeout'
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let retest_count = timeout_records.len();
        let mut all_mutants = untested_mutants;

        // Append timeout mutants to the list (prioritizing no-outcome mutants first)
        for r in timeout_records {
            all_mutants.push(Mutant {
                id: r.id,
                target_id: r.target_id,
                byte_offset: r.byte_offset as u32,
                line_offset: r.line_offset as u32,
                old_text: r.old_text,
                new_text: r.new_text,
                mutation_slug: r.mutation_slug,
            });
        }

        Ok((all_mutants, untested_count, retest_count))
    }

    pub async fn get_mutant_test_counts(&self, target_id: i64) -> StoreResult<(usize, usize)> {
        let mutants = self.get_mutants(target_id).await?;
        let mut untested_count = 0;
        let mut retest_count = 0;

        for mutant in &mutants {
            match self.get_outcome(mutant.id).await {
                Ok(None) => untested_count += 1,
                Ok(Some(outcome)) if outcome.status == Status::Timeout => retest_count += 1,
                _ => {}
            }
        }

        Ok((untested_count, retest_count))
    }

    /// Get campaign-wide statistics about mutation testing results
    pub async fn get_campaign_summary(&self) -> StoreResult<CampaignSummary> {
        // Get status counts using a SQL query for efficiency
        let records = sqlx::query!(
            r#"
            SELECT status, COUNT(*) as count
            FROM outcomes
            GROUP BY status
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut caught = 0; // TestFail + BuildFail
        let mut uncaught = 0; // Uncaught
        let mut skipped = 0; // Skipped

        for record in records {
            let count = record.count as usize;
            match record.status.as_str() {
                "TestFail" | "BuildFail" => caught += count,
                "Uncaught" => uncaught += count,
                "Skipped" => skipped += count,
                "Timeout" => {
                    // Timeouts are not conclusive, don't count them
                }
                _ => {}
            }
        }

        let tested = caught + uncaught;

        Ok(CampaignSummary {
            tested,
            caught,
            uncaught,
            skipped,
        })
    }
}
