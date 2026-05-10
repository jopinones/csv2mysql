// loader.rs
use crate::error::Result;
use crate::types::{DataRow, ExecutionMode, LoadStats, TableSchema};
use crate::proc_log::ProcessLogger;
use sqlx::{MySql, Pool, Transaction};
use std::time::Instant;
use tracing::{info, warn};

pub struct Loader {
    pool: Pool<MySql>,
    batch_size: usize,
}

impl Loader {
    pub fn new(pool: Pool<MySql>, batch_size: usize) -> Self {
        Self { pool, batch_size }
    }

    pub async fn load(
        &self,
        schema: &TableSchema,
        rows: &[DataRow],
        mode: ExecutionMode,
        process_id: i64,
    ) -> Result<LoadStats> {
        let start = Instant::now();
        let total = rows.len() as u64;
        
        if mode == ExecutionMode::DryRun || mode == ExecutionMode::Validate {
            info!("Modo {}: simulando carga de {} filas", mode, total);
            return Ok(LoadStats {
                rows_read: total,
                rows_inserted: 0,
                rows_rejected: 0,
                warnings: 0,
                duration_ms: start.elapsed().as_millis() as u64,
                throughput: 0.0,
            });
        }

        let mut tx = self.pool.begin().await?;
        let mut inserted = 0u64;

        for chunk in rows.chunks(self.batch_size) {
            let sql = self.build_insert_statement(schema, chunk);
            match sqlx::query(&sql).execute(&mut *tx).await {
                Ok(result) => {
                    inserted += result.rows_affected();
                    info!(
                        "Batch insertado: {} filas (total: {}/{})",
                        result.rows_affected(),
                        inserted,
                        total
                    );
                }
                Err(e) => {
                    warn!("Error en batch: {}", e);
                    tx.rollback().await?;
                    return Err(e.into());
                }
            }
        }

        tx.commit().await?;
        
        let duration = start.elapsed().as_millis() as u64;
        let throughput = if duration > 0 {
            (inserted as f64 / duration as f64) * 1000.0
        } else {
            0.0
        };

        Ok(LoadStats {
            rows_read: total,
            rows_inserted: inserted,
            rows_rejected: total - inserted,
            warnings: 0,
            duration_ms: duration,
            throughput,
        })
    }

    fn build_insert_statement(&self, schema: &TableSchema, rows: &[DataRow]) -> String {
        let columns: Vec<String> = schema.columns.iter().map(|c| c.name.clone()).collect();
        let col_list = columns.join(", ");

        let values: Vec<String> = rows
            .iter()
            .map(|row| {
                let vals: Vec<String> = schema
                    .columns
                    .iter()
                    .map(|col| {
                        row.values
                            .get(&col.csv_name)
                            .and_then(|v| v.as_ref())
                            .map(|v| format!("'{}'", v.replace('\'', "''")))
                            .unwrap_or_else(|| "NULL".to_string())
                    })
                    .collect();
                format!("({})", vals.join(", "))
            })
            .collect();

        format!(
            "INSERT INTO {} ({}) VALUES {}",
            schema.table_name,
            col_list,
            values.join(", ")
        )
    }
}
