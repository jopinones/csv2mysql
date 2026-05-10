use crate::error::Result;
use crate::types::{ExecutionMode, ProcessLog, ProcessStatus};
use chrono::Utc;
use sqlx::{MySql, Pool};

pub struct ProcessLogger {
    pool: Pool<MySql>,
}

impl ProcessLogger {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }

    pub async fn create(&self, log: &ProcessLog) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO proceso_log (
                archivo_nombre, archivo_ruta, archivo_hash, tabla_destino,
                config_usado, modo, estado, filas_leidas, filas_insertadas,
                filas_rechazadas, warnings, inicio, usuario_so, host
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            log.archivo_nombre,
            log.archivo_ruta,
            log.archivo_hash,
            log.tabla_destino,
            log.config_usado,
            log.modo.to_string(),
            log.estado.to_string(),
            log.filas_leidas,
            log.filas_insertadas,
            log.filas_rechazadas,
            log.warnings,
            log.inicio,
            log.usuario_so,
            log.host,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_id() as i64)
    }

    pub async fn update(
        &self,
        id: i64,
        estado: ProcessStatus,
        filas_insertadas: i64,
        error: Option<String>,
    ) -> Result<()> {
        let fin = Utc::now();
        
        sqlx::query!(
            r#"
            UPDATE proceso_log 
            SET estado = ?, filas_insertadas = ?, fin = ?, error_mensaje = ?
            WHERE id = ?
            "#,
            estado.to_string(),
            filas_insertadas,
            fin,
            error,
            id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
